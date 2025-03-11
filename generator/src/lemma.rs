use crate::turtle::graph::Turtle;
use crate::util::lazy_async::CloneError;
use crate::PACKAGE_PATH;
use anyhow::{anyhow, Context};
use safe_once_async::detached::{spawn_transparent, JoinTransparent};
use safe_once_async::sync::AsyncLazyLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use tokio::fs;

#[derive(Debug)]
pub struct Lemma {
    forward: HashMap<String, Vec<String>>,
    reverse: HashMap<String, Vec<String>>,
}

impl Lemma {
    pub async fn from_file(path: &Path) -> anyhow::Result<Self> {
        Ok(Self::parse(
            &fs::read_to_string(path)
                .await
                .with_context(|| format!("while reading {:?}", path))?,
        )
        .with_context(|| format!("while parsing {:?}", path))?)
    }
    pub fn parse(contents: &str) -> anyhow::Result<Self> {
        let mut forward: HashMap<String, Vec<String>> = HashMap::new();
        let mut reverse: HashMap<String, Vec<String>> = HashMap::new();
        for line in contents.split('\n') {
            if line.starts_with(";") || line.is_empty() {
                continue;
            }
            let (canon, alts) = line.split_once("->").ok_or_else(|| anyhow!("missing ->"))?;
            let canon = canon.split("/").next().unwrap();
            for alt in alts.split(",") {
                reverse
                    .entry(alt.to_string())
                    .or_default()
                    .push(canon.to_string());
                forward
                    .entry(canon.to_string())
                    .or_default()
                    .push(alt.to_string());
            }
        }
        Ok(Self { forward, reverse })
    }
    pub fn alternates(&self, s: &str) -> &[String] {
        if let Some(forward) = self.forward.get(s) {
            forward
        } else {
            &[]
        }
    }
    pub fn canonicals(&self, s: &str) -> &[String] {
        if let Some(reverse) = self.reverse.get(s) {
            reverse
        } else {
            &[]
        }
    }
}

pub static LEMMA: LazyLock<AsyncLazyLock<JoinTransparent<anyhow::Result<Arc<Lemma>>>>> =
    LazyLock::new(|| {
        AsyncLazyLock::new(spawn_transparent(async move {
            Ok(Arc::new(Lemma::from_file(&PACKAGE_PATH.join("build/lemma.en.txt")).await?))
        }))
    });

#[tokio::test]
async fn test_lemma() -> anyhow::Result<()> {
    let lemma = LEMMA.get().await.clone_error()?;
    Ok(())
}
