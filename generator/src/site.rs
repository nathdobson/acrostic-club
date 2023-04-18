use std::io;
use std::ffi::OsString;
use std::io::{ErrorKind, Write};
use std::path::Path;

use futures::future::BoxFuture;
use futures::FutureExt;
use serde::Deserialize;
use serde::Serialize;
use tokio::fs;
use tokio::fs::{create_dir, create_dir_all, read_dir};
use tokio::task::JoinHandle;

use crate::{PACKAGE_PATH, write_path};

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
    let mut f = &mut f;
    writeln!(f, "<!DOCTYPE html>")?;
    writeln!(f, "<html>")?;
    writeln!(f, "<head>")?;
    writeln!(f, "<title>Acrostic Puzzles</title>")?;
    writeln!(f, "</head>")?;
    writeln!(f, "<body>")?;
    let mut d = tokio::fs::read_dir(PACKAGE_PATH.join("build/site/puzzles")).await?;
    while let Some(e) = d.next_entry().await? {
        let filename = e.file_name();
        let name = filename.to_str().unwrap();
        index.links.push(PuzzleLink {
            index:
            name.strip_prefix("puzzle").unwrap().strip_suffix(".json").unwrap().parse().unwrap(),
            name: name.to_string().strip_suffix(".json").unwrap().to_string(),
            url: format!("./puzzles/{}", name),
        });
        writeln!(
            f,
            "<p><a href=\"./index.html?puzzle=./puzzles/{}\" >{}</a></p>",
            name,
            name,
        )?;
    }
    index.links.sort();
    writeln!(f, "<p><a href=\"./ACKNOWLEDGEMENTS.txt\">ACKNOWLEDGEMENTS</a></p>")?;
    writeln!(f, "<p><a href=\"./LICENSE.txt\">LICENSE</a></p>")?;
    writeln!(f, "</body>")?;
    writeln!(f, "</html>")?;
    // write_path(&PACKAGE_PATH.join("build/site/index.html"), f).await?;
    write_path(&PACKAGE_PATH.join("build/site/puzzles.json"), serde_json::to_string(&index)?.as_bytes()).await?;
    Ok(())
}

pub fn copy_dir<'a>(source: &'a Path, dest: &'a Path) -> BoxFuture<'a, io::Result<()>> {
    async move {
        create_dir_all(dest).await?;
        let mut dir = read_dir(source).await?;
        // let mut tasks: Vec<JoinHandle<io::Result<()>>> = vec![];
        while let Some(entry) = dir.next_entry().await? {
            // tasks.push(tokio::spawn(async move {
            let metadata = entry.metadata().await?;
            let dest = dest.join(entry.file_name());
            if metadata.is_dir() {
                copy_dir(&entry.path(), &dest).await?;
            } else if metadata.is_file() {
                fs::symlink(entry.path(), dest).await?;
            } else {
                return Err(io::Error::new(ErrorKind::Unsupported, "Not a file or directory"));
            }
            // Ok(())
            // }));
        }
        // for task in tasks {
        //     task.await?;
        // }
        Ok(())
    }.boxed()
}

pub async fn copy_puzzles() -> io::Result<()> {
    let mut dir = read_dir(PACKAGE_PATH.join("puzzles")).await?;
    let output = PACKAGE_PATH.join("build/site/puzzles");
    create_dir_all(&output).await?;
    while let Some(entry) = dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        if metadata.is_dir() {
            let mut filename = OsString::new();
            filename.push("puzzle");
            filename.push(entry.file_name());
            filename.push(".json");
            let input = entry.path().join("stage4.json");
            if fs::metadata(&input).await.is_ok() {
                if let Err(e) = fs::symlink(input,
                                            output.join(filename)).await {
                    if e.kind() != ErrorKind::NotFound {
                        return Err(e);
                    }
                };
            }
        }
    }
    Ok(())
}

pub async fn build_site() -> io::Result<()> {
    fs::remove_dir_all(&PACKAGE_PATH.join("build/site")).await?;
    copy_dir(&PACKAGE_PATH.join("src/player"), &PACKAGE_PATH.join("build/site")).await?;
    fs::symlink(&PACKAGE_PATH.join("LICENSE"), &PACKAGE_PATH.join("build/site/LICENSE.txt")).await?;
    fs::symlink(&PACKAGE_PATH.join("ACKNOWLEDGEMENTS"), &PACKAGE_PATH.join("build/site/ACKNOWLEDGEMENTS.txt")).await?;
    copy_puzzles().await?;
    build_index().await?;
    Ok(())
}