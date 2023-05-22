use std::any::{Any, type_name};
use std::collections::{HashMap, HashSet};
use std::default::default;
use std::fmt::{Debug, Formatter};
use std::io;
use std::marker::PhantomData;
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
use safe_once_async::sync::AsyncLazyStatic;
use ustr::Ustr;
use acrostic_core::letter::Letter;
use crate::segment::get_alpha;
use crate::turtle::{TURTLE, Turtle, TurtleIndex};
use crate::util::lazy_async::CloneError;

pub struct Ontology {
    graph: &'static Turtle,
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

impl Ontology {
    pub async fn new() -> anyhow::Result<Self> {
        let graph = TURTLE.get().await.clone_error()?;
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
    pub fn canonical_form_of(&self, x: Form) -> Vec<Lexical> {
        self.graph.get_reverse(x.0, self.canonical_form).into_iter().map(Lexical).collect()
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
    pub fn etym_related_to(&self, x: Etymology) -> Vec<Etymology> {
        self.graph.get_forward(x.0, self.etym_related).into_iter().map(Etymology).collect()
    }
    pub fn derived_from(&self, x: Page) -> Vec<Lexical> {
        self.graph.get_forward(x.0, self.derived_from).into_iter().map(Lexical).collect()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Written(TurtleIndex);

#[derive(Debug, Copy, Clone)]
pub struct Page(TurtleIndex);

#[derive(Debug, Copy, Clone)]
pub struct Lexical(TurtleIndex);

#[derive(Debug, Copy, Clone)]
pub struct Form(TurtleIndex);

#[derive(Debug, Copy, Clone)]
pub struct Etymology(TurtleIndex);

#[derive(Debug, Copy, Clone)]
pub struct Sense(TurtleIndex);

#[derive(Debug, Copy, Clone)]
pub struct Translation(TurtleIndex);

#[derive(Debug, Copy, Clone)]
pub struct Other(TurtleIndex);

pub static ONTOLOGY: AsyncLazyStatic<anyhow::Result<Ontology>> = AsyncLazyStatic::new_static(async move {
    Ok(Ontology::new().await?)
});

#[tokio::test]
async fn read_turtle_graph() -> anyhow::Result<()> {
    ONTOLOGY.get().await.clone_error()?;
    Ok(())
}

//
// #[derive(Debug)]
// pub struct Graph {
//     entries: HashMap<Ustr, NodeData>,
//     // forms: HashMap<Vec<Letter>, Vec<Ustr>>,
// }
//
// impl NodeData {
//     pub fn parse(&mut self) {
//         let types = self.edges.get(&Ustr::from("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")).cloned().unwrap_or(vec![]);
//         if types.contains(&Ustr::from("http://www.w3.org/ns/lemon/ontolex#Form")) {
//             self.typ = Box::new(Form);
//         } else if types.contains(&Ustr::from("http://www.w3.org/ns/lemon/ontolex#LexicalEntry")) {
//             self.typ = Box::new(Lexical);
//         } else if types.contains(&Ustr::from("http://kaiko.getalp.org/dbnary#Page")) {
//             self.typ = Box::new(Page);
//         } else if types.contains(&Ustr::from("http://etytree-virtuoso.wmflabs.org/dbnaryetymology#EtymologyEntry")) {
//             self.typ = Box::new(Etymology);
//         } else if types.contains(&Ustr::from("http://www.w3.org/ns/lemon/ontolex#LexicalSense")) {
//             self.typ = Box::new(Sense);
//         } else if types.contains(&Ustr::from("http://kaiko.getalp.org/dbnary#Translation")) {
//             self.typ = Box::new(Translation);
//         } else {
//             self.typ = Box::new(Other);
//         }
//     }
// }
//
// impl<'a> NodeRef<'a, Page> {
//     fn describes(&self) -> Vec<NodeRef<'a, Lexical>> {
//         self.forward("http://kaiko.getalp.org/dbnary#describes")
//     }
// }
//
// impl<'a> NodeRef<'a, Lexical> {
//     fn describes_of(&self) -> Vec<NodeRef<'a, Page>> {
//         self.reverse("http://kaiko.getalp.org/dbnary#describes")
//     }
//     fn canonical_form(&self) -> Vec<NodeRef<'a, Form>> {
//         self.forward("http://www.w3.org/ns/lemon/ontolex#canonicalForm")
//     }
//     fn other_forms(&self) -> Vec<NodeRef<'a, Form>> {
//         self.forward("http://www.w3.org/ns/lemon/ontolex#otherForm")
//     }
// }
//
// impl<'a> NodeRef<'a, Form> {
//     fn canonical_form_of(&self) -> Vec<NodeRef<'a, Lexical>> {
//         self.reverse("http://www.w3.org/ns/lemon/ontolex#canonicalForm")
//     }
//     fn other_form_of(&self) -> Vec<NodeRef<'a, Lexical>> {
//         self.reverse("http://www.w3.org/ns/lemon/ontolex#otherForm")
//     }
//     fn letters(&self) -> Vec<Vec<Letter>> {
//         self.get("http://www.w3.org/ns/lemon/ontolex#writtenRep").iter().map(|x| get_alpha(&*x)).collect()
//     }
// }
//
// impl<'a, T: 'static> NodeRef<'a, T> {
//     pub fn forward<T2: 'static>(&self, name: &str) -> Vec<NodeRef<'a, T2>> {
//         self.1.edges.get(&Ustr::from(name))
//             .unwrap_or(&vec![])
//             .iter()
//             .map(|x| self.0.find(*x).unwrap())
//             .collect()
//     }
//     pub fn reverse<T2: 'static>(&self, name: &str) -> Vec<NodeRef<'a, T2>> {
//         self.1.in_edges.get(&Ustr::from(name))
//             .unwrap_or(&vec![])
//             .iter()
//             .map(|x| self.0.find(*x).unwrap())
//             .collect()
//     }
//     pub fn get(&self, name: &str) -> &[Ustr] {
//         match self.1.edges.get(&Ustr::from(name)) {
//             None => &[],
//             Some(x) => x
//         }
//     }
// }
//
// impl Graph {
//     fn parse(buffers: &[&str]) -> Self {
//         let mut graph = Graph { entries: default() };
//         for buffer in buffers {
//             let mut parser = TurtleParser::new(buffer.as_ref(), None);
//             // for i in .. {
//             parser.parse_all(&mut |t| {
//                 let subject = match t.subject {
//                     Subject::NamedNode(node) => Ustr::from(node.iri),
//                     Subject::BlankNode(x) => { Ustr::from(x.id) }
//                     _ => panic!("{:?}", t.subject),
//                 };
//                 let entry = graph.entries.entry(subject).or_insert_with(|| NodeData {
//                     typ: Box::new(()),
//                     subject,
//                     edges: Default::default(),
//                     in_edges: default(),
//                 });
//                 let object = match t.object {
//                     Term::Literal(literal) => {
//                         match literal {
//                             Literal::Simple { value } => Ustr::from(value),
//                             Literal::LanguageTaggedString { value, language } => Ustr::from(value),
//                             Literal::Typed { value, .. } => Ustr::from(value),
//                         }
//                     }
//                     Term::NamedNode(x) => Ustr::from(x.iri),
//                     Term::BlankNode(x) => Ustr::from(x.id),
//                     _ => panic!("{:?}", t.object)
//                 };
//                 entry.edges.entry(Ustr::from(t.predicate.iri)).or_default().push(object);
//                 Ok(()) as Result<(), TurtleError>
//             }).unwrap();
//             // }
//         }
//         let mut edges: Vec<_> = vec![];
//         for entry in &mut graph.entries {
//             entry.1.parse();
//             edges.push((*entry.0, entry.1.edges.clone()));
//         }
//         for (from, edges) in edges {
//             for (typ, to) in edges {
//                 for to in to {
//                     if let Some(to) = graph.entries.get_mut(&to) {
//                         to.in_edges.entry(typ).or_default().push(from);
//                     }
//                 }
//             }
//         }
//         let mut forms: HashMap<Vec<Letter>, Vec<_>> = HashMap::new();
//         for form in graph.iter::<Form>() {
//             for letters in form.letters() {
//                 forms.entry(letters).or_default().push(form.1.subject);
//             }
//         }
//         // graph.forms = forms;
//         // let pe = graph.find::<Page>(Ustr::from("http://kaiko.getalp.org/dbnary/eng/dictionary")).unwrap();
//         // let le = pe.describes().into_iter().next().unwrap();
//         // let cf = le.canonical_form().into_iter().next().unwrap();
//         // println!("{:?}", graph.variants(get_alpha("dictionary")));
//         // println!("{:?}", graph.variants(get_alpha("dictionaries")));
//         // println!("{:?}", graph.variants(get_alpha("cat")));
//         // println!("{:?}", graph.variants(get_alpha("cats")));
//         // println!("{:?}", graph.variants(get_alpha("dog")));
//         // println!("{:?}", graph.variants(get_alpha("dogs")));
//         // println!("{:?}", graph.variants(get_alpha("abide")));
//         // let le = graph.forward(pe, DESCRIBES).into_iter().next().unwrap();
//         // let se = graph.forward(le, SENSE).into_iter().next().unwrap();
//
//         // println!("{:?}", pe.1);
//         // println!("{:#?}", graph.entries.get(&Ustr::from("http://kaiko.getalp.org/dbnary/eng/dictionary")));
//         // println!("{:#?}", graph.entries.get(&Ustr::from("http://kaiko.getalp.org/dbnary/eng/dictionary__Noun__1")));
//         graph
//     }
//     fn find<T: 'static>(&self, name: Ustr) -> Option<NodeRef<T>> {
//         let node = self.entries.get(&name)?;
//         (&*node.typ as &dyn Any).downcast_ref::<T>()?;
//         Some(NodeRef(self, node, PhantomData))
//     }
//     fn iter<T: 'static>(&self) -> impl Iterator<Item=NodeRef<T>> {
//         self.entries.iter().filter_map(|x| self.find(*x.0))
//     }
//     // fn variants(&self, str: Vec<Letter>) -> Vec<Vec<Letter>> {
//     //     let mut result = HashSet::<Vec<Letter>>::new();
//     //     for form in self.forms.get(&str).unwrap() {
//     //         let form = self.find::<Form>(*form).unwrap();
//     //         for lexical in form.canonical_form_of().into_iter().chain(form.other_form_of().into_iter()) {
//     //             for page in lexical.describes_of() {
//     //                 for l2 in page.describes() {
//     //                     for f2 in l2.canonical_form().into_iter().chain(l2.other_forms().into_iter()) {
//     //                         for ls2 in f2.letters() {
//     //                             result.insert(ls2);
//     //                         }
//     //                     }
//     //                 }
//     //             }
//     //         }
//     //     }
//     //     result.into_iter().collect()
//     // }
// }
//
// #[tokio::test]
// async fn test() -> anyhow::Result<()> {
//     // let ontolex = fs::read_to_string(&PACKAGE_PATH.join("build/en_dbnary_ontolex.ttl")).await?;
//     // let morphology = fs::read_to_string(&PACKAGE_PATH.join("build/en_dbnary_morphology.ttl")).await?;
//     // let graph = Graph::parse(&[&ontolex,&morphology]);
//     // Ok(())
// }

#[tokio::test]
async fn test_ontology() -> anyhow::Result<()> {
    let ontology = ONTOLOGY.get().await.clone_error()?;
    let rep = ontology.find_written("netball").unwrap();
    dbg!(ontology.graph.debug(rep.0));
    for form in ontology.written_rep_of(rep) {
        dbg!(ontology.graph.debug(form.0));
        for cf in ontology.canonical_form_of(form) {
            dbg!(ontology.graph.debug(cf.0));
            let (ees, ps) = ontology.describes_of(cf);
            for ee in ees {
                dbg!(ontology.graph.debug(ee.0));
                for ee2 in ontology.etym_related_to(ee){
                    dbg!(ontology.graph.debug(ee2.0));
                }
            }
            for p in ps {
                dbg!(ontology.graph.debug(p.0));
                for df in ontology.derived_from(p){
                    dbg!(ontology.graph.debug(df.0));
                }
            }
        }
    }

    Ok(())
}