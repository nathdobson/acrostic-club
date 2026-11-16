// Tests for `ONTOLOGY`: loads the ontology from `build/`.

use acrostic_generator::ontology::ONTOLOGY;
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
#[ignore = "needs the generator's outputs in build/"]
async fn the_ontology_loads() -> anyhow::Result<()> {
    ONTOLOGY.get().await.clone_error()?;
    Ok(())
}

// The commented-out test below doesn't compile: it calls `get_directed_conflicts`, which
// `Ontology` lacks, and passes a string to `find_written`, which takes a `LetterString`.
// #[tokio::test]
// async fn test_ontology() -> anyhow::Result<()> {
//     let ontology = ONTOLOGY.get().await.clone_error()?;
//     let word = "islamist";
//     println!("{:?}", ontology.get_directed_conflicts(word));
//     let rep = ontology.find_written("netball").unwrap();
//
//     dbg!(ontology.graph.debug(rep.0));
//     for form in ontology.written_rep_of(rep) {
//         dbg!(ontology.graph.debug(form.0));
//         for cf in ontology.canonical_form_of(form) {
//             dbg!(ontology.graph.debug(cf.0));
//             let (ees, ps) = ontology.describes_of(cf);
//             for ee in ees {
//                 dbg!(ontology.graph.debug(ee.0));
//                 for ee2 in ontology.etym_related_to(ee) {
//                     dbg!(ontology.graph.debug(ee2.0));
//                 }
//             }
//             for p in ps {
//                 dbg!(ontology.graph.debug(p.0));
//                 for df in ontology.derived_from(p) {
//                     dbg!(ontology.graph.debug(df.0));
//                 }
//             }
//         }
//     }
//
//     Ok(())
// }
