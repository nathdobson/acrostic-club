// Tests for parsing the WordNet turtle file from `build/`: prints the entries for "buy" and
// "bought".

use acrostic_generator::PACKAGE_PATH;
use acrostic_generator::turtle::parse::parse_file_graph;

#[tokio::test]
async fn the_wordnet_graph_has_entries_for_buy_and_bought() -> anyhow::Result<()> {
    let graph = parse_file_graph(&[&PACKAGE_PATH.join("build/english-wordnet-2024.ttl")]).await?;

    let buy = graph
        .get_index("buy")
        .expect("WordNet should have an entry for buy");
    let bought = graph
        .get_index("bought")
        .expect("WordNet should have an entry for bought");

    let buy_entry = graph.debug(buy);
    let bought_entry = graph.debug(bought);
    println!("{buy_entry:?} {bought_entry:?}");
    Ok(())
}
