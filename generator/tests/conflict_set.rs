// Tests for `ConflictSet`: prints the conflicts that a few words produce. Reads the ontology from
// `build/`.

use acrostic_generator::conflict_set::ConflictSet;
use acrostic_generator::ontology::ONTOLOGY;
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
async fn conflict_sets_build_for_five_words() -> anyhow::Result<()> {
    let ontology = ONTOLOGY.get().await.clone_error()?.clone();

    for word in ["bought", "definition", "cowboy", "cattle", "Islam"] {
        let mut conflicts = ConflictSet::new(ontology.clone());
        conflicts.add_origin(word.to_string());

        println!("{conflicts:#?}");
        let terminals = conflicts.terminals().collect::<Vec<_>>();
        println!("{terminals:#?}");
    }
    Ok(())
}
