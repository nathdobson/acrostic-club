use std::collections::{BTreeSet, HashMap, HashSet};
use std::default::default;
use std::{any, io};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::sync::LazyLock;
use rio_api::model::{Literal, Subject, Term};
use rio_api::parser::TriplesParser;
use rio_turtle::{TurtleError, TurtleParser};
use safe_once_async::sync::AsyncLazyStatic;
use tokio::fs;
use crate::PACKAGE_PATH;
use serde::Serialize;
use serde::Deserialize;
use crate::util::lazy_async::CloneError;
// use crate::util::lazy_async::LazyAsync;

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
    pub async fn new() -> anyhow::Result<Self> {
        let ontolex = fs::read_to_string(&PACKAGE_PATH.join("build/en_dbnary_ontolex.ttl")).await?;
        let morphology = fs::read_to_string(&PACKAGE_PATH.join("build/en_dbnary_morphology.ttl")).await?;
        let etymology = fs::read_to_string(&PACKAGE_PATH.join("build/en_dbnary_etymology.ttl")).await?;
        let mut triples = vec![];
        for buffer in &[ontolex, morphology, etymology] {
            let mut parser = TurtleParser::new(buffer.as_ref(), None);
            parser.parse_all(&mut |t| {
                let subject = match t.subject {
                    Subject::NamedNode(node) => node.iri,
                    Subject::BlankNode(x) => { x.id }
                    _ => panic!("{:?}", t.subject),
                };
                let predicate = t.predicate.iri;
                let object = match t.object {
                    Term::Literal(literal) => {
                        match literal {
                            Literal::Simple { value } => value,
                            Literal::LanguageTaggedString { value, language } => value,
                            Literal::Typed { value, .. } => value,
                        }
                    }
                    Term::NamedNode(x) => x.iri,
                    Term::BlankNode(x) => x.id,
                    _ => panic!("{:?}", t.object)
                };
                triples.push((subject.to_string(), predicate.to_string(), object.to_string()));
                Ok(()) as Result<(), TurtleError>
            })?;
        }
        let id_set = triples.iter().flat_map(
            |(x, y, z)| [&**x, &**y, &**z].into_iter()
        ).collect::<HashSet<&str>>();
        let mut ids: Vec<String> = id_set.into_iter().map(|x| x.to_string()).collect();
        ids.sort();
        let mut id_map = HashMap::new();
        for (index, id) in ids.iter().enumerate() {
            id_map.insert(id.clone(), TurtleIndex(index as u32));
        }
        let mut forward = vec![];
        let mut reverse = vec![];
        for (s, p, o) in triples {
            let (s, p, o) = (*id_map.get(&s).unwrap(), *id_map.get(&p).unwrap(), *id_map.get(&o).unwrap());
            forward.push((s, p, o));
            reverse.push((o, p, s));
        }
        forward.sort();
        reverse.sort();
        Ok(Turtle { ids, forward: EdgeList(forward), reverse: EdgeList(reverse) })
    }
    pub fn get_name(&self, index: TurtleIndex) -> &str {
        &self.ids[index.0 as usize]
    }
    pub fn get_index(&self, name: &str) -> Option<TurtleIndex> {
        Some(TurtleIndex(self.ids.binary_search_by(|other: &String| (&**other).cmp(name)).ok()? as u32))
    }
    pub fn debug<'a>(&'a self, index: TurtleIndex) -> TurtleDebug<'a> {
        TurtleDebug(self, index)
    }
    pub fn get_forward(&self, s: TurtleIndex, p: TurtleIndex) -> Vec<TurtleIndex> {
        self.forward.get_edges(s, p)
    }
    pub fn get_reverse(&self, o: TurtleIndex, p: TurtleIndex) -> Vec<TurtleIndex> {
        self.reverse.get_edges(o, p)
    }
}

fn binary_search_range<A, B: Ord, F: FnMut(&A) -> B>(slice: &[A], pivot: B, mut get: F) -> &[A] {
    let lower_bound =
        slice.binary_search_by(|other| match get(other).cmp(&pivot) {
            Ordering::Equal => Ordering::Greater,
            ord => ord,
        }).unwrap_err();
    let upper_bound =
        slice.binary_search_by(|other| match get(other).cmp(&pivot) {
            Ordering::Equal => Ordering::Less,
            ord => ord,
        }).unwrap_err();
    &slice[lower_bound..upper_bound]
}


impl EdgeList {
    pub fn get_node(&self, index: TurtleIndex) -> Vec<(TurtleIndex, TurtleIndex)> {
        binary_search_range(&self.0, index, |x| x.0)
            .into_iter().map(|(a, b, c)| (*b, *c)).collect()
    }
    pub fn get_edges(&self, index: TurtleIndex, pred: TurtleIndex) -> Vec<TurtleIndex> {
        binary_search_range(&self.0, (index, pred),
                            |(a, b, c)| (*a, *b))
            .into_iter().map(|(a, b, c)| *c).collect()
    }
}


pub async fn build_turtle() -> anyhow::Result<()> {
    let turtle = Turtle::new().await?;
    fs::write(PACKAGE_PATH.join("build/turtle.dat"), bincode::serialize(&turtle)?).await?;
    Ok(())
}

pub static TURTLE: AsyncLazyStatic<anyhow::Result<Turtle>> = AsyncLazyStatic::new_static(async move {
    Ok(bincode::deserialize(&fs::read(PACKAGE_PATH.join("build/turtle.dat")).await?)?)
});

impl<'a> Debug for TurtleDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut m = f.debug_map();
        m.entry(&"index", &self.1);
        m.entry(&"name", &self.0.get_name(self.1));
        for (p, o) in self.0.forward.get_node(self.1) {
            m.entry(&self.0.get_name(p), &self.0.get_name(o));
        }
        for (p, s) in self.0.reverse.get_node(self.1) {
            m.entry(&format!("reverse {:?}", self.0.get_name(p)), &self.0.get_name(s));
        }
        m.finish()
    }
}

#[tokio::test]
async fn read_turtle2() -> anyhow::Result<()> {
    TURTLE.get().await.clone_error()?;
    Ok(())
}

#[test]
fn test_binary_search() {
    assert_eq!([(0, "a"), (0, "b")], binary_search_range(&[(0, "a"), (0, "b"), (1, "c"), (1, "d")], 0, |x| x.0));
    assert_eq!([(1, "c"), (1, "d")], binary_search_range(&[(0, "a"), (0, "b"), (1, "c"), (1, "d")], 1, |x| x.0));
}