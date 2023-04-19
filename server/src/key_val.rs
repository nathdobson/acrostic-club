use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use serde::Serialize;
use serde::Deserialize;

#[derive(Default, Clone, Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Debug)]
struct Seq {
    time: u64,
    breaker: u64,
}

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct SeqValue<V> {
    #[serde(flatten)]
    seq: Seq,
    #[serde(flatten)]
    value: V,
}

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct KeyVal<K: Eq + Hash, V>(HashMap<K, SeqValue<V>>);

impl<K: Eq + Hash, V> KeyVal<K, V> {
    pub fn merge_from(&mut self, other: &Self) where K: Clone, V: Clone {
        for (k, v) in &other.0 {
            match self.0.entry(k.clone()) {
                Entry::Occupied(mut e) => {
                    if e.get().seq < v.seq {
                        e.insert(v.clone());
                    }
                }
                Entry::Vacant(e) => {
                    e.insert(v.clone());
                }
            }
        }
    }
}

#[test]
fn test() {
    assert_eq!(serde_json::to_string(&KeyVal::<usize, usize>::default()).unwrap(), "{}");
}