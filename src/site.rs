use std::{io};
use std::ffi::OsString;
use std::path::Path;
use crate::PACKAGE_PATH;
use std::io::{ErrorKind, Write};
use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::fs;
use tokio::fs::{create_dir, create_dir_all, read_dir};
use tokio::task::JoinHandle;

async fn build_index() -> io::Result<()> {
    let mut f = vec![];
    let mut f = &mut f;
    writeln!(f, "<!DOCTYPE html>")?;
    writeln!(f, "<html>")?;
    writeln!(f, "<head>")?;
    writeln!(f, "<title>Acrostic Puzzles</title>")?;
    writeln!(f, "</head>")?;
    writeln!(f, "<body>")?;
    let mut d = tokio::fs::read_dir(PACKAGE_PATH.join("src/public/")).await?;
    while let Some(e) = d.next_entry().await? {
        writeln!(
            f,
            "<p><a href=\"./puzzle.html?puzzle=./puzzles/{}\" >{}</a></p>",
            e.file_name().to_str().unwrap(),
            e.file_name().to_str().unwrap()
        )?;
    }
    writeln!(f, "</body>")?;
    writeln!(f, "</html>")?;
    tokio::fs::write(PACKAGE_PATH.join("public/index.html"), f).await?;
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
                fs::copy(entry.path(), dest).await?;
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
            if let Err(e) = fs::copy(input,
                                     output.join(filename)).await {
                if e.kind() != ErrorKind::NotFound {
                    return Err(e);
                }
            };
        }
    }
    Ok(())
}

pub async fn build_site() -> io::Result<()> {
    fs::remove_dir_all(&PACKAGE_PATH.join("build/site")).await?;
    copy_dir(&PACKAGE_PATH.join("src/player"), &PACKAGE_PATH.join("build/site")).await?;
    copy_puzzles().await?;
    Ok(())
}