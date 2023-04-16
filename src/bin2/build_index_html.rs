#![allow(unused_variables, unused_mut)]
#![deny(unused_must_use)]

use std::io;
use std::io::Write;

use acrostic::PACKAGE_PATH;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut f = vec![];
    let mut f = &mut f;
    writeln!(f, "<!DOCTYPE html>")?;
    writeln!(f, "<html>")?;
    writeln!(f, "<head>")?;
    writeln!(f, "<title>Acrostic Puzzles</title>")?;
    writeln!(f, "</head>")?;
    writeln!(f, "<body>")?;
    let mut d = tokio::fs::read_dir(PACKAGE_PATH.join("public/puzzles/")).await?;
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
