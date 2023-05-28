use std::any::{Any, type_name};
use std::collections::{HashMap, HashSet};
use std::default::default;
use std::fmt::{Debug, Formatter};
use std::io;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::fs;
// use turtle_syntax::{Document, Parse};
use crate::PACKAGE_PATH;
use codespan_reporting;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use futures::StreamExt;
use itertools::Itertools;
use locspan::Meta;
use rio_api::model::{Literal, NamedNode, Subject, Term};
use rio_turtle::{TurtleError, TurtleParser};
use rio_api::parser::TriplesParser;
use safe_once_async::sync::AsyncStaticLock;
use ustr::Ustr;
use acrostic_core::letter::Letter;
use crate::conflict_set::ConflictSet;
use crate::segment::get_alpha;
use crate::turtle::{TURTLE, Turtle, TurtleIndex};
use crate::util::lazy_async::CloneError;

pub struct Ontology {
    pub graph: &'static Turtle,
    other_form: TurtleIndex,
    written_rep: TurtleIndex,
    canonical_form: TurtleIndex,
    describes: TurtleIndex,
    typ: TurtleIndex,
    type_etymology: TurtleIndex,
    type_page: TurtleIndex,
    etym_related: TurtleIndex,
    derived_from: TurtleIndex,
}

impl Debug for Ontology {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ontology").finish()
    }
}

impl Ontology {
    pub async fn new() -> anyhow::Result<Self> {
        let graph = TURTLE.get().await.clone_error_static()?;
        Ok(Ontology {
            other_form: graph.get_index("http://www.w3.org/ns/lemon/ontolex#otherForm").unwrap(),
            written_rep: graph.get_index("http://www.w3.org/ns/lemon/ontolex#writtenRep").unwrap(),
            canonical_form: graph.get_index("http://www.w3.org/ns/lemon/ontolex#canonicalForm").unwrap(),
            describes: graph.get_index("http://kaiko.getalp.org/dbnary#describes").unwrap(),
            typ: graph.get_index("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
            type_etymology: graph.get_index("http://etytree-virtuoso.wmflabs.org/dbnaryetymology#EtymologyEntry").unwrap(),
            type_page: graph.get_index("http://kaiko.getalp.org/dbnary#Page").unwrap(),
            etym_related: graph.get_index("http://etytree-virtuoso.wmflabs.org/dbnaryetymology#etymologicallyRelatedTo").unwrap(),
            derived_from: graph.get_index("http://kaiko.getalp.org/dbnary#derivedFrom").unwrap(),
            graph,
        })
    }
    pub fn find_written(&self, text: &str) -> Option<Written> {
        Some(Written(self.graph.get_index(text)?))
    }
    pub fn written_rep_of(&self, x: Written) -> Vec<Form> {
        self.graph.get_reverse(x.0, self.written_rep).into_iter().map(Form).collect()
    }
    pub fn written_rep(&self, x: Form) -> Vec<Written> {
        self.graph.get_forward(x.0, self.written_rep).into_iter().map(Written).collect()
    }
    pub fn canonical_form_of(&self, x: Form) -> Vec<Lexical> {
        self.graph.get_reverse(x.0, self.canonical_form).into_iter().map(Lexical).collect()
    }
    pub fn canonical_form(&self, x: Lexical) -> Vec<Form> {
        self.graph.get_forward(x.0, self.canonical_form).into_iter().map(Form).collect()
    }
    pub fn other_form_of(&self, x: Form) -> Vec<Lexical> {
        self.graph.get_reverse(x.0, self.other_form).into_iter().map(Lexical).collect()
    }
    pub fn other_form(&self, x: Lexical) -> Vec<Form> {
        self.graph.get_forward(x.0, self.other_form).into_iter().map(Form).collect()
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
                eprintln!("{:?}", self.graph.debug(i));
            }
        }
        (ees, ps)
    }
    pub fn describes_etym(&self, x: Etymology) -> Vec<Lexical> {
        self.graph.get_forward(x.0, self.describes).into_iter().map(Lexical).collect()
    }
    pub fn describes_page(&self, x: Page) -> Vec<Lexical> {
        self.graph.get_forward(x.0, self.describes).into_iter().map(Lexical).collect()
    }
    pub fn etym_related_to(&self, x: Etymology) -> Vec<Etymology> {
        self.graph.get_forward(x.0, self.etym_related).into_iter().map(Etymology).collect()
    }
    pub fn derived_from(&self, x: Page) -> Vec<Lexical> {
        self.graph.get_forward(x.0, self.derived_from).into_iter().map(Lexical).collect()
    }
    pub fn derived_from_of(&self, x: Lexical) -> Vec<Page> {
        self.graph.get_reverse(x.0, self.derived_from).into_iter().map(Page).collect()
    }
    pub fn get_conflicts(self: &Arc<Self>, x: &str) -> Vec<String> {
        let mut set = ConflictSet::new(self.clone());
        set.add_origin(x.to_string());
        set.terminals().map(|x| x.to_string()).collect()
    }
    // pub fn get_conflict_keys(&self, x: &str) -> Vec<&str> {
    //     let rep = self.find_written(x).unwrap();
    //     let forms = self.written_rep_of(rep).into_iter().collect::<HashSet<_>>();
    //     println!("{:?}", self.graph.debug_all(forms.iter().map(|x| x.0)));
    //     let les = forms.into_iter().flat_map(|f| self.canonical_form_of(f)).collect::<HashSet<_>>();
    //     println!("{:?}", self.graph.debug_all(les.iter().map(|x| x.0)));
    //     let mut ees = HashSet::new();
    //     let mut ps = HashSet::new();
    //     for le in les {
    //         let (ees1, ps1) = self.describes_of(le);
    //         for ee in ees1 { ees.insert(ee); }
    //         for p in ps1 { ps.insert(p); }
    //     }
    //     println!("{:?}", self.graph.debug_all(ees.iter().map(|x| x.0)));
    //     println!("{:?}", self.graph.debug_all(ps.iter().map(|x| x.0)));
    //     let ers = ees.into_iter().flat_map(|ee| self.etym_related_to(ee)).collect::<HashSet<_>>();
    //     println!("{:?}", self.graph.debug_all(ers.iter().map(|x| x.0)));
    //     let les2 = ps.into_iter().flat_map(|p| self.derived_from(p)).collect::<HashSet<_>>();
    //     println!("{:?}", self.graph.debug_all(ps.iter().map(|x| x.0)));
    //
    //     // les.into_iter().chain(les2)
    // }
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

pub static ONTOLOGY: AsyncStaticLock<anyhow::Result<Arc<Ontology>>> = AsyncStaticLock::new(async move {
    Ok(Arc::new(Ontology::new().await?))
});

#[tokio::test]
async fn read_turtle_graph() -> anyhow::Result<()> {
    ONTOLOGY.get().await.clone_error()?;
    Ok(())
}

#[tokio::test]
async fn test_ontology() -> anyhow::Result<()> {
    let ontology = ONTOLOGY.get().await.clone_error()?;
    // println!("{:?}", ontology.get_directed_conflicts("netball"));
    // let rep = ontology.find_written("netball").unwrap();

    // dbg!(ontology.graph.debug(rep.0));
    // for form in ontology.written_rep_of(rep) {
    //     dbg!(ontology.graph.debug(form.0));
    //     for cf in ontology.canonical_form_of(form) {
    //         dbg!(ontology.graph.debug(cf.0));
    //         let (ees, ps) = ontology.describes_of(cf);
    //         for ee in ees {
    //             dbg!(ontology.graph.debug(ee.0));
    //             for ee2 in ontology.etym_related_to(ee) {
    //                 dbg!(ontology.graph.debug(ee2.0));
    //             }
    //         }
    //         for p in ps {
    //             dbg!(ontology.graph.debug(p.0));
    //             for df in ontology.derived_from(p) {
    //                 dbg!(ontology.graph.debug(df.0));
    //             }
    //         }
    //     }
    // }


    Ok(())
}