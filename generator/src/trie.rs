use futures::TryStreamExt;
use futures::stream::FuturesUnordered;
use std::alloc::Allocator;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::mem::size_of;
use std::ops::RangeInclusive;
use std::path::Path;
use std::{io, mem};

use acrostic_core::letter::{Letter, LetterMap, LetterSet};
use anyhow::Context;

use crate::PACKAGE_PATH;
use crate::dict::FLAT_WORDS;
use crate::util::alloc::{MmapAllocator, restore_vec, save_vec};
use crate::util::parallel::Parallelism;

#[repr(C)]
pub enum FlatTrieEntry<V> {
    Leaf { key: LetterSet, value: V },
    Node { letter: Letter, second: usize },
}

#[repr(transparent)]
pub struct FlatTrie<V>([FlatTrieEntry<V>]);

pub enum FlatTrieView<'a, V> {
    Empty,
    Leaf {
        key: &'a LetterSet,
        value: &'a V,
        remainder: &'a FlatTrie<V>,
    },
    Node {
        letter: Letter,
        without: &'a FlatTrie<V>,
        with: &'a FlatTrie<V>,
    },
}

impl<V: Clone> FlatTrie<V> {
    pub fn new_unchecked_box<A: Allocator + Sized>(b: Box<[FlatTrieEntry<V>], A>) -> Box<Self, A> {
        unsafe {
            assert_eq!(
                size_of::<Box<[FlatTrieEntry<V>], A>>(),
                size_of::<Box<Self, A>>(),
            );
            let result = mem::transmute_copy(&b);
            mem::forget(b);
            result
        }
    }
    pub async unsafe fn restore(filename: &Path) -> io::Result<Box<Self, MmapAllocator>> {
        let vec: Box<[FlatTrieEntry<V>], MmapAllocator> = restore_vec(filename).await?;
        Ok(Self::new_unchecked_box(vec))
    }
    pub fn new_unchecked_ref(b: &[FlatTrieEntry<V>]) -> &Self {
        unsafe { mem::transmute(b) }
    }
    pub fn new_unchecked_mut(b: &mut [FlatTrieEntry<V>]) -> &mut Self {
        unsafe { mem::transmute(b) }
    }
    pub fn as_slice(&self) -> &[FlatTrieEntry<V>] {
        &self.0
    }
    pub fn view(&self) -> FlatTrieView<'_, V> {
        match &self.0.split_first() {
            None => FlatTrieView::Empty,
            Some((FlatTrieEntry::Leaf { key, value }, remainder)) => FlatTrieView::Leaf {
                key,
                value,
                remainder: Self::new_unchecked_ref(remainder),
            },
            Some((FlatTrieEntry::Node { letter, second }, remainder)) => FlatTrieView::Node {
                letter: *letter,
                without: Self::new_unchecked_ref(&remainder[..*second]),
                with: Self::new_unchecked_ref(&remainder[*second..]),
            },
        }
    }
    pub fn search_exact(&self, k: LetterSet) -> Option<&V> {
        match self.view() {
            FlatTrieView::Empty => None,
            FlatTrieView::Leaf {
                key,
                value,
                remainder,
            } => {
                if *key == k {
                    Some(value)
                } else {
                    remainder.search_exact(k)
                }
            }
            FlatTrieView::Node {
                letter,
                without,
                with,
            } => {
                if k[letter] > 0 {
                    let mut k2 = k;
                    k2[letter] -= 1;
                    with.search_exact(k2)
                } else {
                    without.search_exact(k)
                }
            }
        }
    }
    #[inline(never)]
    pub fn search_smallest_subset(&self, key: LetterSet, min_len: usize, result: &mut Vec<V>) {
        for len in min_len..=key.count() {
            self.search_subsets_with_length_in(key, len..=len, result);
        }
    }
    // Finds the values of the longest keys that are subsets of `superset` and hold at most
    // `max_length` letters, in trie order.
    pub fn search_largest_subset(
        &self,
        superset: LetterSet,
        max_length: usize,
        found: &mut Vec<V>,
    ) {
        // Try each length from the longest down, and stop at the first that any key reaches.
        for length in (0..=max_length).rev() {
            self.search_subsets_with_length_in(superset, length..=length, found);
            if !found.is_empty() {
                return;
            }
        }
    }

    // Finds the values of every key that is a subset of `superset` and holds at least `min_length`
    // letters, in a single walk of the trie. `search_smallest_subset` returns the same values
    // grouped from shortest to longest, at the cost of one walk per length.
    pub fn search_subsets_with_min_length(
        &self,
        superset: LetterSet,
        min_length: usize,
        found: &mut Vec<V>,
    ) {
        self.search_subsets_with_length_in(superset, min_length..=superset.count(), found);
    }

    // Finds the values of every key that is a subset of `superset` and whose length lies in
    // `lengths`, in trie order.
    fn search_subsets_with_length_in(
        &self,
        superset: LetterSet,
        lengths: RangeInclusive<usize>,
        found: &mut Vec<V>,
    ) {
        self.walk_subsets_with_length_in(
            superset,
            *lengths.start(),
            *lengths.end(),
            0,
            superset.count(),
            found,
        );
    }

    // Walks the part of the trie that `self` holds, for `search_subsets_with_length_in`, accepting
    // keys of `min_length` to `max_length` letters. `letters_matched` counts the key letters that
    // the path down to this part has matched. `letters_reachable` is that count plus the letters
    // still available in `superset`: the most letters any key in this part can hold.
    fn walk_subsets_with_length_in(
        &self,
        superset: LetterSet,
        min_length: usize,
        max_length: usize,
        letters_matched: usize,
        letters_reachable: usize,
        found: &mut Vec<V>,
    ) {
        match self.view() {
            FlatTrieView::Empty => {}
            FlatTrieView::Leaf {
                key,
                value,
                remainder,
            } => {
                if key.is_subset(superset) {
                    // A leaf holds only the key letters that its path hasn't matched.
                    let length = letters_matched + key.count();
                    if (min_length..=max_length).contains(&length) {
                        found.push(value.clone());
                    }
                }
                remainder.walk_subsets_with_length_in(
                    superset,
                    min_length,
                    max_length,
                    letters_matched,
                    letters_reachable,
                    found,
                )
            }
            FlatTrieView::Node {
                letter,
                without,
                with,
            } => {
                // The keys on the "without" side hold no more copies of `letter` than the path has
                // matched, so the superset's remaining copies are out of their reach. Skip that
                // side when its keys can't reach the shortest length allowed.
                let remaining_copies = superset[letter] as usize;
                let letters_reachable_without = letters_reachable - remaining_copies;
                if letters_reachable_without >= min_length {
                    let mut superset_without = superset;
                    superset_without[letter] = 0;
                    without.walk_subsets_with_length_in(
                        superset_without,
                        min_length,
                        max_length,
                        letters_matched,
                        letters_reachable_without,
                        found,
                    );
                }
                // The keys on the "with" side hold at least one more copy of `letter`, which this
                // step matches. Skip that side when the superset has no copy left to match, or when
                // its keys would pass the longest length allowed.
                if remaining_copies > 0 && letters_matched < max_length {
                    let mut superset_with = superset;
                    superset_with[letter] -= 1;
                    with.walk_subsets_with_length_in(
                        superset_with,
                        min_length,
                        max_length,
                        letters_matched + 1,
                        letters_reachable,
                        found,
                    );
                }
            }
        }
    }

    // pub fn new_box<A: Allocator>(mut b: Box<[FlatTrieEntry<K, V>], A>) -> Box<Self, A> {
    //     Self::build(&mut *b);
    //     Self::new_unchecked_box(b)
    // }
    // pub fn new_mut(mut b: &mut [FlatTrieEntry<K, V>]) -> &mut Self {
    //     Self::build(&mut *b);
    //     Self::new_unchecked_mut(b)
    // }
    // fn build(this: &mut [FlatTrieEntry<K, V>]) { todo!() }
}

struct FlatTrieBuilder<V> {
    output: Vec<FlatTrieEntry<V>>,
}

impl<V: Debug> FlatTrieBuilder<V> {
    fn new() -> FlatTrieBuilder<V> {
        FlatTrieBuilder { output: vec![] }
    }
    fn add_leaves(&mut self, leaves: &mut [(LetterSet, Option<V>)], prefix: LetterSet) {
        for x in leaves {
            self.output.push(FlatTrieEntry::Leaf {
                key: x.0 - prefix,
                value: x.1.take().unwrap(),
            });
        }
    }
    fn add_entries(&mut self, leaves: &mut [(LetterSet, Option<V>)], mut prefix: LetterSet) {
        if leaves.len() <= 1 {
            self.add_leaves(leaves, prefix);
        } else {
            let mut totals = LetterMap::<u32>::new();
            for (k, _) in leaves.iter() {
                for l in Letter::all() {
                    if k[l] > prefix[l] {
                        totals[l] += 1;
                    }
                }
            }
            let split = totals.iter().max_by_key(|x| x.1).unwrap().0;
            let (left, right) = partition::partition(leaves, |x| x.0[split] == prefix[split]);
            if right.len() == 0 {
                self.add_leaves(left, prefix);
                return;
            }
            let node_index = self.output.len();
            self.output.push(FlatTrieEntry::Node {
                letter: split,
                second: usize::MAX,
            });
            self.add_entries(left, prefix);
            self.output[node_index] = FlatTrieEntry::Node {
                letter: split,
                second: self.output.len() - node_index - 1,
            };
            prefix[split] += 1;
            self.add_entries(right, prefix);
        }
    }
}

impl<V: Debug + Clone> FromIterator<(LetterSet, V)> for Box<FlatTrie<V>> {
    fn from_iter<T: IntoIterator<Item = (LetterSet, V)>>(iter: T) -> Self {
        let mut entries = iter
            .into_iter()
            .map(|(k, v)| (k, Some(v)))
            .collect::<Vec<_>>();
        // let mut rand = XorShiftRng::seed_from_u64(123);
        // entries.shuffle(&mut rand);
        // let mut entries = entries.into_iter().collect::<HashMap<_, _>>().into_iter().collect::<Vec<_>>();
        let mut builder = FlatTrieBuilder::new();
        builder.add_entries(&mut entries, LetterSet::new());
        let result = FlatTrie::new_unchecked_box(builder.output.into_boxed_slice());
        // println!("{:?}", result);
        result
    }
}

impl<V: Debug + Clone> Debug for FlatTrie<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.view() {
            FlatTrieView::Empty => write!(f, "ø"),
            FlatTrieView::Leaf {
                key,
                value,
                remainder,
            } => write!(f, "{:?} = {:?}; {:?}", key, value, remainder),
            FlatTrieView::Node {
                letter,
                without,
                with,
            } => {
                write!(f, "{{ {:?} }}; {:?}  {{ {:?} }}", without, letter, with)
            }
        }
    }
}

impl<V: Debug> Debug for FlatTrieEntry<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FlatTrieEntry::Leaf { key, value } => f
                .debug_struct("")
                .field("key", &key)
                .field("value", &value)
                .finish(),
            FlatTrieEntry::Node { letter, second } => f
                .debug_struct("")
                .field("letter", &letter)
                .field("second", &second)
                .finish(),
        }
    }
}

pub async fn build_trie() -> anyhow::Result<()> {
    // Create the output folders, which a fresh checkout lacks. Doing so before the long
    // computation below also surfaces a failure early.
    let unary_folder = PACKAGE_PATH.join("build/unary");
    let binary_folder = PACKAGE_PATH.join("build/binary");
    for folder in [&unary_folder, &binary_folder] {
        tokio::fs::create_dir_all(folder)
            .await
            .with_context(|| format!("Cannot create {folder:?}"))?;
    }

    let dict = FLAT_WORDS.get_static().await?;
    let mut binary = BTreeMap::<(Letter, Letter), Vec<(LetterSet, (LetterSet, LetterSet))>>::new();
    let mut unary = BTreeMap::<Letter, Vec<(LetterSet, LetterSet)>>::new();
    for l in Letter::all() {
        unary.insert(l, vec![]);
    }
    for l1 in Letter::all() {
        for l2 in Letter::all() {
            binary.insert((l1, l2), vec![]);
        }
    }
    let mut words = vec![];
    for word in dict.iter() {
        if word.letters.count() > 5 {
            words.push(word);
        }
    }
    let words = &words[0..15000];
    for word1 in words {
        if let Some(first1) = word1.letter_vec.first() {
            unary
                .entry(*first1)
                .or_default()
                .push((word1.letters, word1.letters));
            for word2 in words {
                if let Some(first2) = word2.letter_vec.first() {
                    if first1 <= first2 {
                        binary.entry((*first1, *first2)).or_default().push((
                            word1.letters + word2.letters,
                            (word1.letters, word2.letters),
                        ));
                    }
                }
            }
        }
    }

    for (l1, vec) in unary {
        println!("{:?}", l1);
        save_vec::<FlatTrieEntry<LetterSet>>(
            &unary_folder.join(format!("map_{l1}.dat")),
            vec.into_iter()
                .collect::<Box<FlatTrie<LetterSet>>>()
                .as_slice(),
        )
        .await?;
    }
    let parallelism = Parallelism::new();
    binary
        .into_iter()
        .map(|((l1, l2), vec)| {
            let parallelism = &parallelism;
            let binary_folder = &binary_folder;
            async move {
                let built: Box<FlatTrie<(LetterSet, LetterSet)>> = parallelism
                    .run_blocking(move || {
                        println!("Computing {:?}/{:?}", l1, l2);
                        vec.into_iter()
                            .collect::<Box<FlatTrie<(LetterSet, LetterSet)>>>()
                    })
                    .await;
                save_vec::<FlatTrieEntry<(LetterSet, LetterSet)>>(
                    &binary_folder.join(format!("map_{l1}_{l2}.dat")),
                    built.as_slice(),
                )
                .await?;
                println!("Done {:?}/{:?}", l1, l2);
                Result::<(), io::Error>::Ok(())
            }
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<()>()
        .await?;
    // let mut tasks = vec![];
    // let semaphore = Semaphore::new(num_cpus::get());
    // for ((l1, l2), vec) in binary {
    //     tasks.push(tokio::task::spawn(|| {
    //         {
    //             let guard = semaphore.acquire();
    //             println!("{:?}/{:?}", l1, l2);
    //
    //         }

    //         ).await?;
    //     }));
    // }
    // for task in tasks {
    //     task.join();
    // }
    Ok(())
}
