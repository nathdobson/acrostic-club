// Tests for `Turtle`'s edge lookups.

use acrostic_generator::turtle::graph::Turtle;

// Builds a graph from (subject, predicate, object) names.
fn build_graph(triples: &[(&str, &str, &str)]) -> Turtle {
    let owned_triples = triples
        .iter()
        .map(|(subject, predicate, object)| {
            (
                subject.to_string(),
                predicate.to_string(),
                object.to_string(),
            )
        })
        .collect();
    Turtle::new(owned_triples).expect("the graph should build from these triples")
}

// Returns the names of the objects that `predicate` links to from `subject`.
fn object_names<'graph>(graph: &'graph Turtle, subject: &str, predicate: &str) -> Vec<&'graph str> {
    let subject_index = graph
        .get_index(subject)
        .expect("the subject should be in the graph");
    let predicate_index = graph
        .get_index(predicate)
        .expect("the predicate should be in the graph");
    graph
        .get_forward(subject_index, predicate_index)
        .into_iter()
        .map(|index| graph.get_name(index))
        .collect()
}

#[test]
fn a_forward_lookup_finds_every_object_of_its_subject() {
    let graph = build_graph(&[
        ("buy", "form", "bought"),
        ("buy", "form", "buying"),
        ("go", "form", "gone"),
        ("go", "form", "went"),
    ]);

    assert_eq!(object_names(&graph, "buy", "form"), ["bought", "buying"]);
    assert_eq!(object_names(&graph, "go", "form"), ["gone", "went"]);
}
