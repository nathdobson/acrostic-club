// Tests for `ONTOLOGY`: loads the ontology from `build/`.

use acrostic_generator::ontology::ONTOLOGY;
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
#[ignore = "needs the generator's outputs in build/"]
async fn the_ontology_loads() -> anyhow::Result<()> {
    ONTOLOGY.get().await.clone_error()?;
    Ok(())
}
