use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};

#[derive(Serialize, Deserialize)]
pub struct Turtle {
    ids: Vec<String>,
    forward: EdgeList,
    reverse: EdgeList,
}

#[derive(Serialize, Deserialize)]
struct EdgeList(Vec<(TurtleIndex, TurtleIndex, TurtleIndex)>);

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TurtleIndex(u32);

pub struct TurtleDebug<'a>(&'a Turtle, TurtleIndex);

impl Turtle {
    pub fn new(triples: Vec<(String, String, String)>) -> anyhow::Result<Self> {
        let id_set = triples
            .iter()
            .flat_map(|(x, y, z)| [&**x, &**y, &**z].into_iter())
            .collect::<HashSet<&str>>();
        let mut ids: Vec<String> = id_set.into_iter().map(|x| x.to_string()).collect();
        ids.sort();
        let mut id_map = HashMap::new();
        for (index, id) in ids.iter().enumerate() {
            id_map.insert(id.clone(), TurtleIndex(index as u32));
        }
        let mut forward = vec![];
        let mut reverse = vec![];
        for (s, p, o) in triples {
            let (s, p, o) = (
                *id_map.get(&s).unwrap(),
                *id_map.get(&p).unwrap(),
                *id_map.get(&o).unwrap(),
            );
            forward.push((s, p, o));
            reverse.push((o, p, s));
        }
        forward.sort();
        reverse.sort();
        Ok(Turtle {
            ids,
            forward: EdgeList(forward),
            reverse: EdgeList(reverse),
        })
    }
    pub fn get_name(&self, index: TurtleIndex) -> &str {
        &self.ids[index.0 as usize]
    }
    pub fn get_index(&self, name: &str) -> Option<TurtleIndex> {
        Some(TurtleIndex(
            self.ids
                .binary_search_by(|other: &String| (&**other).cmp(name))
                .ok()? as u32,
        ))
    }
    pub fn debug<'a>(&'a self, index: TurtleIndex) -> TurtleDebug<'a> {
        TurtleDebug(self, index)
    }
    pub fn debug_all<'a>(
        &'a self,
        input: impl IntoIterator<Item = TurtleIndex>,
    ) -> Vec<TurtleDebug<'a>> {
        input.into_iter().map(|x| self.debug(x)).collect()
    }
    pub fn get_forward(&self, s: TurtleIndex, p: TurtleIndex) -> Vec<TurtleIndex> {
        self.forward.get_edges(s, p)
    }
    pub fn get_reverse(&self, o: TurtleIndex, p: TurtleIndex) -> Vec<TurtleIndex> {
        self.reverse.get_edges(o, p)
    }
    pub fn get_edges_by_predicate(&self, p: TurtleIndex) -> Vec<(TurtleIndex, TurtleIndex)> {
        self.forward.get_edges_by_pred(p)
    }
}

fn binary_search_range<A, B: Ord, F: FnMut(&A) -> B>(slice: &[A], pivot: B, mut get: F) -> &[A] {
    let lower_bound = slice
        .binary_search_by(|other| match get(other).cmp(&pivot) {
            Ordering::Equal => Ordering::Greater,
            ord => ord,
        })
        .unwrap_err();
    let upper_bound = slice
        .binary_search_by(|other| match get(other).cmp(&pivot) {
            Ordering::Equal => Ordering::Less,
            ord => ord,
        })
        .unwrap_err();
    &slice[lower_bound..upper_bound]
}

impl EdgeList {
    pub fn get_node(&self, index: TurtleIndex) -> Vec<(TurtleIndex, TurtleIndex)> {
        binary_search_range(&self.0, index, |x| x.0)
            .into_iter()
            .map(|(a, b, c)| (*b, *c))
            .collect()
    }
    pub fn get_edges(&self, index: TurtleIndex, pred: TurtleIndex) -> Vec<TurtleIndex> {
        binary_search_range(&self.0, (index, pred), |(a, b, c)| (*a, *b))
            .into_iter()
            .map(|(a, b, c)| *c)
            .collect()
    }
    pub fn get_edges_by_pred(&self, pred: TurtleIndex) -> Vec<(TurtleIndex, TurtleIndex)> {
        self.0
            .iter()
            .filter_map(|&(s, p, o)| (pred == p).then_some((s, o)))
            .collect()
    }
}
impl<'a> Debug for TurtleDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut m = f.debug_map();
        m.entry(&"index", &self.1);
        m.entry(&"name", &self.0.get_name(self.1));
        // for (p, o) in self.0.forward.get_node(self.1) {
        //     m.entry(&self.0.get_name(p), &self.0.get_name(o));
        // }
        // for (p, s) in self.0.reverse.get_node(self.1) {
        //     m.entry(&format!("reverse {:?}", self.0.get_name(p)), &self.0.get_name(s));
        // }
        m.finish()
    }
}

#[test]
fn test_binary_search() {
    assert_eq!(
        [(0, "a"), (0, "b")],
        binary_search_range(&[(0, "a"), (0, "b"), (1, "c"), (1, "d")], 0, |x| x.0)
    );
    assert_eq!(
        [(1, "c"), (1, "d")],
        binary_search_range(&[(0, "a"), (0, "b"), (1, "c"), (1, "d")], 1, |x| x.0)
    );
}
