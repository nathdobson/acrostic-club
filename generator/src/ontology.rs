use crate::conflict_set::ConflictSet;
use crate::string::LetterString;
use crate::turtle::db::TURTLE;
use crate::turtle::graph::{Turtle, TurtleIndex};
use crate::util::lazy_async::CloneError;
use safe_once::sync::LazyLock;
use safe_once_async::detached::{JoinTransparent, spawn_transparent};
use safe_once_async::sync::AsyncLazyLock;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

pub struct Ontology {
    pub graph: &'static Turtle,
    // None when the dictionary has no word-form links (`ontolex:otherForm`), as in DBnary's
    // English export as of November 2026.
    other_form: Option<TurtleIndex>,
    written_rep: TurtleIndex,
    canonical_form: TurtleIndex,
    describes: TurtleIndex,
    typ: TurtleIndex,
    type_etymology: TurtleIndex,
    type_page: TurtleIndex,
    etym_related: TurtleIndex,
    derived_from: TurtleIndex,
    written_table: HashMap<LetterString, Vec<Written>>,
}

impl Debug for Ontology {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ontology").finish()
    }
}

impl Ontology {
    pub async fn new() -> anyhow::Result<Self> {
        let graph = TURTLE.get().await.clone_error_static()?;
        let written_rep = graph
            .get_index("http://www.w3.org/ns/lemon/ontolex#writtenRep")
            .unwrap();
        let mut written_table = HashMap::<LetterString, Vec<Written>>::new();
        for (_, o) in graph.get_edges_by_predicate(written_rep) {
            let name = graph.get_name(o);
            written_table
                .entry(LetterString::from_str(name))
                .or_default()
                .push(Written(o));
        }
        Ok(Ontology {
            other_form: graph.get_index("http://www.w3.org/ns/lemon/ontolex#otherForm"),
            written_rep,
            canonical_form: graph
                .get_index("http://www.w3.org/ns/lemon/ontolex#canonicalForm")
                .unwrap(),
            describes: graph
                .get_index("http://kaiko.getalp.org/dbnary#describes")
                .unwrap(),
            typ: graph
                .get_index("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                .unwrap(),
            type_etymology: graph
                .get_index("http://etytree-virtuoso.wmflabs.org/dbnaryetymology#EtymologyEntry")
                .unwrap(),
            type_page: graph
                .get_index("http://kaiko.getalp.org/dbnary#Page")
                .unwrap(),
            etym_related: graph
                .get_index(
                    "http://etytree-virtuoso.wmflabs.org/dbnaryetymology#etymologicallyRelatedTo",
                )
                .unwrap(),
            derived_from: graph
                .get_index("http://kaiko.getalp.org/dbnary#derivedFrom")
                .unwrap(),
            graph,
            written_table,
        })
    }
    pub fn find_written(&self, text: &LetterString) -> Vec<Written> {
        if let Some(x) = self.written_table.get(text) {
            x.clone()
        } else {
            vec![]
        }
    }
    pub fn written_rep_of(&self, x: Written) -> Vec<Form> {
        self.graph
            .get_reverse(x.0, self.written_rep)
            .into_iter()
            .map(Form)
            .collect()
    }
    pub fn written_rep(&self, x: Form) -> Vec<Written> {
        self.graph
            .get_forward(x.0, self.written_rep)
            .into_iter()
            .map(Written)
            .collect()
    }
    pub fn canonical_form_of(&self, x: Form) -> Vec<Lexical> {
        self.graph
            .get_reverse(x.0, self.canonical_form)
            .into_iter()
            .map(Lexical)
            .collect()
    }
    pub fn canonical_form(&self, x: Lexical) -> Vec<Form> {
        self.graph
            .get_forward(x.0, self.canonical_form)
            .into_iter()
            .map(Form)
            .collect()
    }
    pub fn other_form_of(&self, x: Form) -> Vec<Lexical> {
        let Some(other_form) = self.other_form else {
            return vec![];
        };
        self.graph
            .get_reverse(x.0, other_form)
            .into_iter()
            .map(Lexical)
            .collect()
    }
    pub fn other_form(&self, x: Lexical) -> Vec<Form> {
        let Some(other_form) = self.other_form else {
            return vec![];
        };
        self.graph
            .get_forward(x.0, other_form)
            .into_iter()
            .map(Form)
            .collect()
    }
    pub fn describes_of(&self, rep: Lexical) -> (Vec<Etymology>, Vec<Page>) {
        let mut ees = vec![];
        let mut ps = vec![];
        for i in self.graph.get_reverse(rep.0, self.describes) {
            let typ = self.graph.get_forward(i, self.typ);
            if typ.contains(&self.type_etymology) {
                ees.push(Etymology(i));
            } else if typ.contains(&self.type_page) {
                ps.push(Page(i));
            } else {
                let unexpected_node = self.graph.debug(i);
                eprintln!("{unexpected_node:?}");
            }
        }
        (ees, ps)
    }
    pub fn describes_etym(&self, x: Etymology) -> Vec<Lexical> {
        self.graph
            .get_forward(x.0, self.describes)
            .into_iter()
            .map(Lexical)
            .collect()
    }
    pub fn describes_page(&self, x: Page) -> Vec<Lexical> {
        self.graph
            .get_forward(x.0, self.describes)
            .into_iter()
            .map(Lexical)
            .collect()
    }
    pub fn etym_related_to(&self, x: Etymology) -> Vec<Etymology> {
        self.graph
            .get_forward(x.0, self.etym_related)
            .into_iter()
            .map(Etymology)
            .collect()
    }
    pub fn derived_from(&self, x: Page) -> Vec<Lexical> {
        self.graph
            .get_forward(x.0, self.derived_from)
            .into_iter()
            .map(Lexical)
            .collect()
    }
    pub fn derived_from_of(&self, x: Lexical) -> Vec<Page> {
        self.graph
            .get_reverse(x.0, self.derived_from)
            .into_iter()
            .map(Page)
            .collect()
    }
    pub fn get_conflicts(self: &Arc<Self>, x: &str) -> Vec<String> {
        let mut set = ConflictSet::new(self.clone());
        set.add_origin(x.to_string());
        set.terminals()
            .map(|x| x.to_string())
            .chain(set.origins().map(|x| x.to_string()))
            .collect()
    }
}

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Written(pub TurtleIndex);

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Page(pub TurtleIndex);

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Lexical(pub TurtleIndex);

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Form(pub TurtleIndex);

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Etymology(pub TurtleIndex);

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Sense(pub TurtleIndex);

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Translation(pub TurtleIndex);

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Other(pub TurtleIndex);

pub static ONTOLOGY: LazyLock<AsyncLazyLock<JoinTransparent<anyhow::Result<Arc<Ontology>>>>> =
    LazyLock::new(|| {
        AsyncLazyLock::new(spawn_transparent(async move {
            Ok(Arc::new(Ontology::new().await?))
        }))
    });
