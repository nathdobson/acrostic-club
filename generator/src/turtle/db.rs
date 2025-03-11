use crate::util::lazy_async::CloneError;
use crate::PACKAGE_PATH;
use anyhow::{anyhow, Context};
// use rio_api::model::{Literal, Subject, Term};
// use rio_api::parser::TriplesParser;
// use rio_turtle::{TurtleError, TurtleParser};
use crate::turtle::graph::{Turtle, TurtleDebug};
use safe_once_async::detached::{spawn_transparent, JoinTransparent};
use safe_once_async::sync::AsyncLazyLock;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::sync::LazyLock;
use std::{any, io};
use tokio::fs;
use crate::turtle::parse::parse_file_graph;

pub async fn build_ontolex_turtle() -> anyhow::Result<()> {
    let turtle = parse_file_graph(&[
        &PACKAGE_PATH.join("build/en_dbnary_ontolex.ttl"),
        &PACKAGE_PATH.join("build/en_dbnary_morphology.ttl"),
        &PACKAGE_PATH.join("build/en_dbnary_etymology.ttl"),
    ])
    .await?;
    fs::write(
        PACKAGE_PATH.join("build/turtle.dat"),
        bincode::serialize(&turtle)?,
    )
    .await?;
    Ok(())
}

pub static TURTLE: LazyLock<AsyncLazyLock<JoinTransparent<anyhow::Result<Turtle>>>> =
    LazyLock::new(|| {
        AsyncLazyLock::new(spawn_transparent(async move {
            Ok(bincode::deserialize(
                &fs::read(PACKAGE_PATH.join("build/turtle.dat")).await?,
            )?)
        }))
    });

