use crate::string::LetterString;
use crate::{PACKAGE_PATH, read_path_to_string};
use anyhow::{Context, anyhow};
use itertools::Itertools;
use safe_once_async::detached::{JoinTransparent, spawn_transparent};
use safe_once_async::sync::AsyncLazyLock;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

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

    // Parses the clue file's tab-separated text: a header row, then one row per clue.
    fn parse(contents: &str) -> anyhow::Result<Self> {
        let mut lines = contents.split('\n');
        lines.next().ok_or_else(|| anyhow!("missing header"))?;
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
        Ok(ClueDb { lookup })
    }
}

pub static CLUE_DB: LazyLock<AsyncLazyLock<JoinTransparent<anyhow::Result<Arc<ClueDb>>>>> =
    LazyLock::new(|| {
        AsyncLazyLock::new(spawn_transparent(async move {
            let path = PACKAGE_PATH.join("build/xd/clues.tsv");
            let contents = read_path_to_string(&path).await?;
            let clue_db =
                ClueDb::parse(&contents).with_context(|| format!("Cannot parse {path:?}"))?;
            Ok(Arc::new(clue_db))
        }))
    });
