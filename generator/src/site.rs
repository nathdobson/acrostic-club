use std::ffi::OsString;
use std::io;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use futures::FutureExt;
use futures::future::BoxFuture;
use serde::Deserialize;
use serde::Serialize;
use tokio::fs;
use tokio::fs::{create_dir_all, read_dir};

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

pub async fn copy_puzzles() -> io::Result<()> {
    let puzzles_folder = PACKAGE_PATH.join("puzzles");
    let mut dir = read_dir(&puzzles_folder)
        .await
        .map_err(error_with_path("list", &puzzles_folder))?;
    let output = PACKAGE_PATH.join("build/site/puzzles");
    create_dir_all(&output)
        .await
        .map_err(error_with_path("create", &output))?;
    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(error_with_path("list", &puzzles_folder))?
    {
        let metadata = entry
            .metadata()
            .await
            .map_err(error_with_path("inspect", &entry.path()))?;
        if metadata.is_dir() {
            let mut filename = OsString::new();
            filename.push("puzzle");
            filename.push(entry.file_name());
            filename.push(".json");
            let input = entry.path().join("stage3.json");
            if fs::metadata(&input).await.is_ok() {
                if let Err(e) = create_link(&input, &output.join(filename)).await {
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
    let site_folder = PACKAGE_PATH.join("build/site");
    // Clear out the previous build, if any: a fresh checkout has none.
    if let Err(error) = fs::remove_dir_all(&site_folder)
        .await
        .map_err(error_with_path("remove", &site_folder))
    {
        if error.kind() != ErrorKind::NotFound {
            return Err(error);
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
    copy_puzzles().await?;
    build_index().await?;
    Ok(())
}
