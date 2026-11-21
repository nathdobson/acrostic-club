use crate::turtle::graph::Turtle;
use crate::turtle::parse::parse_file_graph;
use crate::{PACKAGE_PATH, read_path, write_path};
use anyhow::Context;
use safe_once_async::detached::{JoinTransparent, spawn_transparent};
use safe_once_async::sync::AsyncLazyLock;
use std::sync::LazyLock;

pub async fn build_ontolex_turtle() -> anyhow::Result<()> {
    let turtle = parse_file_graph(&[
        &PACKAGE_PATH.join("build/en_dbnary_ontolex.ttl"),
        &PACKAGE_PATH.join("build/en_dbnary_morphology.ttl"),
        &PACKAGE_PATH.join("build/en_dbnary_etymology.ttl"),
    ])
    .await?;
    write_path(
        &PACKAGE_PATH.join("build/turtle.dat"),
        &bincode::serialize(&turtle)?,
    )
    .await?;
    Ok(())
}

pub static TURTLE: LazyLock<AsyncLazyLock<JoinTransparent<anyhow::Result<Turtle>>>> =
    LazyLock::new(|| {
        AsyncLazyLock::new(spawn_transparent(async move {
            let path = PACKAGE_PATH.join("build/turtle.dat");
            let turtle = bincode::deserialize(&read_path(&path).await?)
                .with_context(|| format!("Cannot parse {path:?}"))?;
            Ok(turtle)
        }))
    });
