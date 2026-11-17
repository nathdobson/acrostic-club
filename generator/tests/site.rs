// Tests for `relative_path`, which lets build/site link to the checkout's files by paths that
// resolve inside whichever checkout holds the links.

use std::io::ErrorKind;
use std::path::Path;

use acrostic_generator::site::relative_path;

#[test]
fn path_climbs_from_the_folder_to_the_shared_prefix() -> anyhow::Result<()> {
    let path = relative_path(
        Path::new("/checkout/player/index.html"),
        Path::new("/checkout/build/site"),
    )?;
    assert_eq!(Path::new("../../player/index.html"), path);
    Ok(())
}

// The generator's paths all start from `generator/..`, so a `..` inside the prefix the two paths
// share must not count as a folder to climb out of.
#[test]
fn dot_dot_inside_the_shared_prefix_is_not_climbed() -> anyhow::Result<()> {
    let path = relative_path(
        Path::new("/checkout/generator/../LICENSE"),
        Path::new("/checkout/generator/../build/site"),
    )?;
    assert_eq!(Path::new("../../LICENSE"), path);
    Ok(())
}

#[test]
fn dot_dot_past_the_shared_prefix_is_refused() {
    let error = relative_path(
        Path::new("/checkout/player/index.html"),
        Path::new("/checkout/build/../build/site"),
    )
    .expect_err("a `..` past the shared prefix should leave no way back to that prefix");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert!(error.to_string().contains("no relative path"), "{error}");
}
