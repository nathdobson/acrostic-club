use std::alloc::Allocator;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::{io, mem};
use std::mem::size_of;
use std::path::Path;
use std::time::Instant;

use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::alloc::{restore_vec, AnyRepr, MmapAllocator};
use crate::{Letter, LetterMap, LetterSet};

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

unsafe impl<V: AnyRepr> AnyRepr for FlatTrieEntry<V> {}

impl<V> FlatTrie<V> {
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
    pub async unsafe fn restore(filename: &Path) -> io::Result<Box<Self, MmapAllocator>>
    where
        V: AnyRepr,
    {
        let vec: Box<[FlatTrieEntry<V>], MmapAllocator> = restore_vec(filename).await?;
        Ok(Self::new_unchecked_box(vec))
    }
    pub fn new_unchecked_ref(b: &[FlatTrieEntry<V>]) -> &Self { unsafe { mem::transmute(b) } }
    pub fn new_unchecked_mut(b: &mut [FlatTrieEntry<V>]) -> &mut Self {
        unsafe { mem::transmute(b) }
    }
    pub fn as_slice(&self) -> &[FlatTrieEntry<V>] { &self.0 }
    pub fn view(&self) -> FlatTrieView<V> {
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
    pub fn search_subset<'a>(
        &'a self,
        superset: LetterSet,
        radius: usize,
        result: &mut Vec<&'a V>,
    ) {
        match self.view() {
            FlatTrieView::Empty => {}
            FlatTrieView::Leaf {
                key,
                value,
                remainder,
            } => {
                if key.is_subset(superset) && (superset - *key).count() == radius {
                    result.push(value)
                }
                remainder.search_subset(superset, radius, result);
            }
            FlatTrieView::Node {
                letter,
                without,
                with,
            } => {
                if let Some(radius2) = radius.checked_sub(superset[letter] as usize) {
                    let mut superset2 = superset;
                    superset2[letter] = 0;
                    without.search_subset(superset2, radius2, result);
                }
                if superset[letter] > 0 {
                    let mut superset2 = superset;
                    superset2[letter] -= 1;
                    with.search_subset(superset2, radius, result);
                }
            }
        }
    }
    pub fn search_smallest_subset(&self, key: LetterSet, min_len: usize) -> Option<&V> {
        for len in min_len..=key.count() {
            let radius = key.count() - len;
            let mut found = vec![];
            self.search_subset(key, radius, &mut found);
            if let Some(found) = found.first() {
                return Some(found);
            }
        }
        None
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
    fn new() -> FlatTrieBuilder<V> { FlatTrieBuilder { output: vec![] } }
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
            for (k, v) in leaves.iter() {
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

impl<V: Debug> FromIterator<(LetterSet, V)> for Box<FlatTrie<V>> {
    fn from_iter<T: IntoIterator<Item = (LetterSet, V)>>(iter: T) -> Self {
        let mut entries = iter
            .into_iter()
            .map(|(k, v)| (k, Some(v)))
            .collect::<Vec<_>>();
        let mut builder = FlatTrieBuilder::new();
        builder.add_entries(&mut entries, LetterSet::new());
        let result = FlatTrie::new_unchecked_box(builder.output.into_boxed_slice());
        format!("{:?}", result);
        result
    }
}

impl<V: Debug> Debug for FlatTrie<V> {
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

#[test]
fn test_flat_trie() {
    let mut entries = vec!["ab", "abc", "abd"];
    let b: Box<FlatTrie<&str>> = entries
        .into_iter()
        .map(|x| (LetterSet::from_str(x), x))
        .collect();
    println!("{:?}", b.as_slice());
    println!("{:?}", b);
}

// #[test]
// fn test_flat_trie_dict() {
//     let start = Instant::now();
//     let x: Box<[FlatTrieEntry<(LetterSet, LetterSet)>], _> =
//         unsafe { restore_vec("data/binary/map_a_a") };
//     println!("{:?}", start.elapsed());
//     let trie = FlatTrie::new_unchecked_ref(&x);
//     let dict = Dict::get();
//     for _ in 0..10000 {
//         let w1 = dict.words.as_slice()[..1000]
//             .choose(&mut thread_rng())
//             .unwrap();
//         let w2 = dict.words.as_slice()[..1000]
//             .choose(&mut thread_rng())
//             .unwrap();
//         if w1.letter_vec[0] == Letter::MIN && w2.letter_vec[0] == Letter::MIN {
//             println!("{:?}", trie.search_exact(w1.letters + w2.letters));
//         }
//     }
// }
