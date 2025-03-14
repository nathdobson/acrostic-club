use crate::lemma::Lemma;
use crate::string::LetterString;
use crate::util::lazy_async::CloneError;
use crate::PACKAGE_PATH;
use anyhow::anyhow;
use itertools::Itertools;
use safe_once_async::detached::{spawn_transparent, JoinTransparent};
use safe_once_async::sync::AsyncLazyLock;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::fs;

#[derive(Debug)]
pub struct ClueEntry {
    pub pubid: String,
    pub year: usize,
    pub answer: String,
    pub clue: String,
}

pub struct ClueDb {
    lookup: HashMap<LetterString, Vec<ClueEntry>>,
}

impl ClueDb {
    pub fn lookup(&self, answer: &LetterString) -> &[ClueEntry] {
        if let Some(entries) = self.lookup.get(answer) {
            entries
        } else {
            &[]
        }
    }
}

pub static CLUE_DB: LazyLock<AsyncLazyLock<JoinTransparent<anyhow::Result<Arc<ClueDb>>>>> =
    LazyLock::new(|| {
        AsyncLazyLock::new(spawn_transparent(async move {
            let contents = fs::read_to_string(PACKAGE_PATH.join("build/xd/clues.tsv")).await?;
            let mut lines = contents.split('\n');
            let header = lines.next().ok_or_else(|| anyhow!("missing header"))?;
            let mut lookup: HashMap<LetterString, Vec<ClueEntry>> = HashMap::new();
            for line in lines {
                let (pubid, year, answer, clue) = line
                    .splitn(4, '\t')
                    .collect_tuple()
                    .ok_or_else(|| anyhow!("not enough cells"))?;
                let entry = ClueEntry {
                    pubid: pubid.to_string(),
                    year: year.parse()?,
                    answer: answer.to_string(),
                    clue: clue.to_string(),
                };
                lookup
                    .entry(LetterString::from_str(&entry.answer))
                    .or_default()
                    .push(entry);
            }
            Ok(Arc::new(ClueDb { lookup }))
        }))
    });

#[tokio::test]
async fn test_cluedb() -> anyhow::Result<()> {
    let clue_db = CLUE_DB.get().await.clone_error_static()?;
    println!("{:?}", clue_db.lookup(&LetterString::from_str("howell")));
    Ok(())
}
