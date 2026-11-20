use std::io;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, anyhow};
use futures::FutureExt;
use futures::future::BoxFuture;
use serde::Deserialize;
use serde::Serialize;
use tokio::fs;
use tokio::fs::{create_dir_all, read_dir};

use crate::cluebank::{ClueBank, clue_bank_path};
use crate::puzzle::{Clue, Puzzle, read_clued_puzzles};
use crate::{PACKAGE_PATH, error_with_path, write_path};

#[derive(Serialize, Deserialize)]
struct PuzzleIndex {
    links: Vec<PuzzleLink>,
}

#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq)]
struct PuzzleLink {
    index: usize,
    name: String,
    url: String,
}

async fn build_index() -> io::Result<()> {
    let mut index = PuzzleIndex { links: vec![] };
    let mut f = vec![];
    let f = &mut f;
    writeln!(f, "<!DOCTYPE html>")?;
    writeln!(f, "<html>")?;
    writeln!(f, "<head>")?;
    writeln!(f, "<title>Acrostic Puzzles</title>")?;
    writeln!(f, "</head>")?;
    writeln!(f, "<body>")?;
    let puzzles_folder = PACKAGE_PATH.join("build/site/puzzles");
    let mut d = tokio::fs::read_dir(&puzzles_folder)
        .await
        .map_err(error_with_path("list", &puzzles_folder))?;
    while let Some(e) = d
        .next_entry()
        .await
        .map_err(error_with_path("list", &puzzles_folder))?
    {
        let filename = e.file_name();
        let name = filename.to_str().unwrap();
        index.links.push(PuzzleLink {
            index: name
                .strip_prefix("puzzle")
                .unwrap()
                .strip_suffix(".json")
                .unwrap()
                .parse()
                .unwrap(),
            name: name.to_string().strip_suffix(".json").unwrap().to_string(),
            url: format!("./puzzles/{}", name),
        });
        writeln!(
            f,
            "<p><a href=\"./index.html?puzzle=./puzzles/{}\" >{}</a></p>",
            name, name,
        )?;
    }
    index.links.sort();
    writeln!(
        f,
        "<p><a href=\"./ACKNOWLEDGEMENTS.txt\">ACKNOWLEDGEMENTS</a></p>"
    )?;
    writeln!(f, "<p><a href=\"./LICENSE.txt\">LICENSE</a></p>")?;
    writeln!(f, "</body>")?;
    writeln!(f, "</html>")?;
    // write_path(&PACKAGE_PATH.join("build/site/index.html"), f).await?;
    write_path(
        &PACKAGE_PATH.join("build/site/puzzles.json"),
        serde_json::to_string(&index)?.as_bytes(),
    )
    .await?;
    Ok(())
}

pub fn copy_dir<'a>(source: &'a Path, dest: &'a Path) -> BoxFuture<'a, io::Result<()>> {
    async move {
        create_dir_all(dest)
            .await
            .map_err(error_with_path("create", dest))?;
        let mut dir = read_dir(source)
            .await
            .map_err(error_with_path("list", source))?;
        // let mut tasks: Vec<JoinHandle<io::Result<()>>> = vec![];
        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(error_with_path("list", source))?
        {
            // tasks.push(tokio::spawn(async move {
            let entry_path = entry.path();
            let metadata = entry
                .metadata()
                .await
                .map_err(error_with_path("inspect", &entry_path))?;
            let dest = dest.join(entry.file_name());
            if metadata.is_dir() {
                copy_dir(&entry_path, &dest).await?;
            } else if metadata.is_file() {
                create_link(&entry_path, &dest).await?;
            } else {
                return Err(io::Error::new(
                    ErrorKind::Unsupported,
                    format!("Not a file or directory: {entry_path:?}"),
                ));
            }
            // Ok(())
            // }));
        }
        // for task in tasks {
        //     task.await?;
        // }
        Ok(())
    }
    .boxed()
}

// Returns the path that reaches `original` from inside `folder`, working on the paths' text alone.
//
// The two paths must share a prefix, past which `folder` names only plain folders: a `..` or a
// root there leaves no way to climb back to the shared prefix.
pub fn relative_path(original: &Path, folder: &Path) -> io::Result<PathBuf> {
    let shared_component_count = original
        .components()
        .zip(folder.components())
        .take_while(|(original_component, folder_component)| original_component == folder_component)
        .count();

    let mut path = PathBuf::new();
    for component in folder.components().skip(shared_component_count) {
        let Component::Normal(_) = component else {
            let step = component.as_os_str();
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "no relative path leads from {folder:?} to {original:?}: the folder takes \
                     the step {step:?} past the prefix the paths share"
                ),
            ));
        };
        // Climb out of each folder between the shared prefix and `folder`.
        path.push("..");
    }
    // Then descend from the shared prefix to the original.
    path.extend(original.components().skip(shared_component_count));
    Ok(path)
}

// Creates a symbolic link at `link` that reaches `original` by a path relative to the link's
// folder. A relative link resolves inside whichever folder holds it, so build/site keeps working
// when build/ is cloned into another checkout, and then serves that checkout's own files.
//
// An error names `link`, since creating a link fails over the link's own location, such as a
// missing parent folder or an existing file.
async fn create_link(original: &Path, link: &Path) -> io::Result<()> {
    let link_folder = link
        .parent()
        .expect("a link's path should name a file inside a folder");
    fs::symlink(relative_path(original, link_folder)?, link)
        .await
        .map_err(error_with_path("create link", link))
}

// A puzzle as the player reads it: the puzzle's stage-3 file, with each clue's tag replaced by the
// clue's text from the clue bank.
#[derive(Serialize, Debug)]
pub struct PublishedPuzzle {
    pub quote: String,
    pub quote_letters: Option<String>,
    pub source: String,
    pub source_letters: Option<String>,
    pub clues: Vec<PublishedClue>,
    pub chat: Option<String>,
}

// One clue as the player reads it.
#[derive(Serialize, Debug)]
pub struct PublishedClue {
    // The clue's text, which the player shows.
    pub clue: String,
    pub answer: String,
    pub answer_letters: String,
    pub indices: Vec<usize>,
}

impl PublishedPuzzle {
    // Builds the published form of `puzzle`, looking up each clue's text in the bank. Fails when
    // the puzzle has no entries yet, when an entry names no clue, or when an entry names a clue
    // the bank lacks.
    pub fn new(puzzle: Puzzle, bank: &ClueBank) -> anyhow::Result<PublishedPuzzle> {
        // Destructuring `Puzzle` and `Clue` in full, rather than reading their fields one by one,
        // makes the compiler flag a field later added to either struct, where the published form
        // would otherwise drop the new field silently.
        let Puzzle {
            quote,
            quote_letters,
            source,
            source_letters,
            clues: entries,
            chat,
        } = puzzle;
        let entries = entries.ok_or_else(|| anyhow!("the puzzle has no entries"))?;
        let published_clues = entries
            .into_iter()
            .map(|entry| {
                let Clue {
                    clue_tag,
                    answer,
                    answer_letters,
                    indices,
                } = entry;
                let tag = clue_tag
                    .as_deref()
                    .ok_or_else(|| anyhow!("the entry for {answer:?} names no clue"))?;
                let banked_clue = bank.find(&answer, tag).ok_or_else(|| {
                    anyhow!("the entry for {answer:?} names clue {tag:?}, which the bank lacks")
                })?;
                Ok(PublishedClue {
                    clue: banked_clue.text().to_string(),
                    answer,
                    answer_letters,
                    indices,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(PublishedPuzzle {
            quote,
            quote_letters,
            source,
            source_letters,
            clues: published_clues,
            chat,
        })
    }
}

// Builds the published form of every puzzle that has a stage-3 file, each paired with its index.
async fn build_published_puzzles() -> anyhow::Result<Vec<(usize, PublishedPuzzle)>> {
    let bank = ClueBank::read(&clue_bank_path()).await?;
    let mut published_puzzles = vec![];
    for (index, puzzle) in read_clued_puzzles().await? {
        let published_puzzle = PublishedPuzzle::new(puzzle, &bank)
            .with_context(|| format!("cannot publish puzzle {index}"))?;
        published_puzzles.push((index, published_puzzle));
    }
    Ok(published_puzzles)
}

// Writes each published puzzle into the site's `puzzles` folder.
async fn write_published_puzzles(
    published_puzzles: &[(usize, PublishedPuzzle)],
) -> anyhow::Result<()> {
    let output = PACKAGE_PATH.join("build/site/puzzles");
    create_dir_all(&output)
        .await
        .map_err(error_with_path("create", &output))?;
    for (index, published_puzzle) in published_puzzles {
        let json = serde_json::to_string_pretty(published_puzzle)
            .expect("a puzzle of plain strings and numbers should always serialize");
        write_path(&output.join(format!("puzzle{index}.json")), json.as_bytes()).await?;
    }
    Ok(())
}

pub async fn build_site() -> anyhow::Result<()> {
    // Every puzzle's published form is built before the previous site is cleared, so bad puzzle or
    // bank data stops the build with the previous site still whole.
    let published_puzzles = build_published_puzzles().await?;

    let site_folder = PACKAGE_PATH.join("build/site");
    // Clear out the previous build, if any: a fresh checkout has none.
    if let Err(error) = fs::remove_dir_all(&site_folder)
        .await
        .map_err(error_with_path("remove", &site_folder))
    {
        if error.kind() != ErrorKind::NotFound {
            return Err(error.into());
        }
    }
    copy_dir(&PACKAGE_PATH.join("player"), &site_folder).await?;
    create_link(
        &PACKAGE_PATH.join("LICENSE"),
        &PACKAGE_PATH.join("build/site/LICENSE.txt"),
    )
    .await?;
    create_link(
        &PACKAGE_PATH.join("ACKNOWLEDGEMENTS"),
        &PACKAGE_PATH.join("build/site/ACKNOWLEDGEMENTS.txt"),
    )
    .await?;
    write_published_puzzles(&published_puzzles).await?;
    build_index().await?;
    Ok(())
}
