use crate::turtle::parse::parse_file_graph;
use crate::PACKAGE_PATH;

#[tokio::test]
async fn test_wordnet_turtle() -> anyhow::Result<()> {
    let turtle = parse_file_graph(&[&PACKAGE_PATH.join("build/english-wordnet-2024.ttl")]).await?;
    let buy = turtle.get_index("buy").unwrap();
    let bought = turtle.get_index("bought").unwrap();
    println!("{:?} {:?}", turtle.debug(buy), turtle.debug(bought));
    Ok(())
}
