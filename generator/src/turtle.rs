use std::collections::HashMap;
use std::default::default;
use std::{any, io};
use std::sync::LazyLock;
use rio_api::model::{Literal, Subject, Term};
use rio_api::parser::TriplesParser;
use rio_turtle::{TurtleError, TurtleParser};
use safe_once_async::sync::AsyncLazyStatic;
use tokio::fs;
use crate::ontology::NodeData;
use crate::PACKAGE_PATH;
use serde::Serialize;
use serde::Deserialize;
// use crate::util::lazy_async::LazyAsync;

#[derive(Serialize, Deserialize)]
pub struct Turtle {
    ids: Vec<String>,
    triples: Vec<(u64, u64, u64)>,
}

impl Turtle {
    pub async fn new() -> anyhow::Result<Self> {
        let ontolex = fs::read_to_string(&PACKAGE_PATH.join("build/en_dbnary_ontolex.ttl")).await?;
        let morphology = fs::read_to_string(&PACKAGE_PATH.join("build/en_dbnary_morphology.ttl")).await?;
        let etymology = fs::read_to_string(&PACKAGE_PATH.join("build/en_dbnary_etymology.ttl")).await?;
        let mut ids = vec![];
        let mut triples = vec![];
        let mut id_map = HashMap::new();
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
                let [subject, predicate, object] = [subject, predicate, object].map(|x| {
                    if let Some(index) = id_map.get(x) {
                        *index
                    } else {
                        let index = ids.len();
                        id_map.insert(x.to_string(), index);
                        ids.push(x.to_string());
                        index
                    }
                });
                Ok(()) as Result<(), TurtleError>
            })?;
        }
        println!("{:?}", ids.len());
        println!("{:?}", ids.iter().map(|x| x.len()).max().unwrap());
        Ok(Turtle { ids, triples })
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

#[tokio::test]
async fn read_turtle() -> anyhow::Result<()> {
    TURTLE.get().await;
    Ok(())
}