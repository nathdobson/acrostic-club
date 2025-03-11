use crate::turtle::graph::Turtle;
use crate::PACKAGE_PATH;
use anyhow::Context;
use std::path::Path;
use oxrdf::{Subject, Term};
use oxttl::TurtleParser;
use tokio::fs;

pub async fn parse_file_graph(paths: &[&Path]) -> anyhow::Result<Turtle> {
    let mut triples = vec![];
    for path in paths {
        triples.extend(parse_file_triples(*path).await?);
    }
    Turtle::new(triples)
}

pub async fn parse_file_triples(path: &Path) -> anyhow::Result<Vec<(String, String, String)>> {
    let contents = fs::read_to_string(path)
        .await
        .with_context(|| format!("while reading path {:?}", path))?;
    let triples =
        parse_triples(&contents).with_context(|| format!("while parsing path {:?}", path))?;
    Ok(triples)
}

pub fn parse_triples(data: &str) -> anyhow::Result<Vec<(String, String, String)>> {
    let mut triples = vec![];
    let mut parser = TurtleParser::new().for_slice(&data.as_bytes());
    for entry in parser {
        let entry = entry?;
        let subject = match &entry.subject {
            Subject::NamedNode(node) => node.as_str(),
            Subject::BlankNode(x) => x.as_str(),
        };
        let predicate = entry.predicate.as_str();
        let object = match &entry.object {
            Term::Literal(literal) => literal.value(),
            Term::NamedNode(x) => x.as_str(),
            Term::BlankNode(x) => x.as_str(),
        };
        triples.push((
            subject.to_string(),
            predicate.to_string(),
            object.to_string(),
        ));
    }
    Ok(triples)
}
