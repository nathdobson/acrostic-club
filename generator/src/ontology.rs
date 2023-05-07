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
use itertools::Itertools;
use locspan::Meta;
use rio_api::model::{Literal, NamedNode, Subject, Term};
use rio_turtle::{TurtleError, TurtleParser};
use rio_api::parser::TriplesParser;
use ustr::Ustr;
use acrostic_core::letter::Letter;
use crate::segment::get_alpha;

#[derive(Debug)]
pub struct NodeData {
    typ: Box<dyn NodeType>,
    subject: Ustr,
    edges: HashMap<Ustr, Vec<Ustr>>,
    in_edges: HashMap<Ustr, Vec<Ustr>>,
}

pub struct NodeRef<'a, T: 'static>(&'a Graph, &'a NodeData, PhantomData<T>);

impl<'a, T: 'static> Debug for NodeRef<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.1.fmt(f)
    }
}

impl<'a, T: 'static> PartialEq<Self> for NodeRef<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.1 as *const NodeData == other.1 as *const NodeData
    }
}

impl<'a, T: 'static> Eq for NodeRef<'a, T> {}

pub trait NodeType: Debug + Any + 'static {}

impl<T: Debug + Any + 'static> NodeType for T {}

#[derive(Debug, Copy, Clone)]
pub struct Page;

#[derive(Debug, Copy, Clone)]
pub struct Lexical;

#[derive(Debug, Copy, Clone)]
pub struct Form;

#[derive(Debug, Copy, Clone)]
pub struct Etymology;

#[derive(Debug, Copy, Clone)]
pub struct Sense;

#[derive(Debug, Copy, Clone)]
pub struct Translation;

#[derive(Debug, Copy, Clone)]
pub struct Other;

#[derive(Copy, Clone)]
pub struct Relation<A, B>(&'static str, PhantomData<(A, B)>);

// #[derive(Copy, Clone)]
// pub struct Field<A>(&'static str, PhantomData<A>);

// impl<A, B> Relation<A, B> {
//     pub const fn new(name: &'static str) -> Self {
//         Relation(name, PhantomData)
//     }
// }

// impl<A> Field<A> {
//     pub const fn new(name: &'static str) -> Self {
//         Field(name, PhantomData)
//     }
// }

// pub static DESCRIBES: Relation<Page, Lexical> = Relation::new("http://kaiko.getalp.org/dbnary#describes");
// pub static LANGUAGE: Field<Lexical> = Field::new("http://purl.org/dc/terms/language");
// pub static SENSE: Relation<Lexical, Sense> = Relation::new("http://www.w3.org/ns/lemon/ontolex#sense");
// pub static PART_OF_SPEECH: Field<Lexical> = Field::new("http://www.lexinfo.net/ontology/2.0/lexinfo#partOfSpeech");
//
// #[derive(Debug)]
// pub struct PageEntry {}
//
// #[derive(Debug)]
// pub struct LexicalEntry {
//     lime_language: Option<Ustr>,
//     language: Option<Ustr>,
// }
//
// #[derive(Debug)]
// pub struct FormEntry {
//     written: Vec<Ustr>,
// }
//
// #[derive(Debug)]
// pub struct EntymologyEntry {}
//
// #[derive(Debug)]
// pub struct SenseEntry {}
//
// #[derive(Debug)]
// pub struct TranslationEntry {}
//
// #[derive(Debug)]
// pub struct OtherEntry {
//     types: Vec<Ustr>,
// }

#[derive(Debug)]
pub struct Graph {
    entries: HashMap<Ustr, NodeData>,
    forms: HashMap<Vec<Letter>, Vec<Ustr>>,
}

impl NodeData {
    // pub fn take_field(&mut self, field: &str) -> Vec<Ustr> {
    //     self.fields.remove(&Ustr::from(field)).unwrap_or(vec![])
    // }
    // pub fn take_exactly_one(&mut self, field: &str) -> Ustr {
    //     let vec = self.take_field(field);
    //     assert_eq!(vec.len(), 1, "{:?}", vec);
    //     *vec.first().unwrap()
    // }
    // pub fn take_at_most_one(&mut self, field: &str) -> Option<Ustr> {
    //     let vec = self.take_field(field);
    //     assert!(vec.len() <= 1, "{:?}", vec);
    //     vec.first().cloned()
    // }
    // fn parse_page(&mut self) {
    //     self.parsed = Box::new(PageEntry {
    //         // describes: self.take_field("http://kaiko.getalp.org/dbnary#describes"),
    //         // derived_from: self.take_field("http://kaiko.getalp.org/dbnary#derivedFrom"),
    //     });
    // }
    // fn parse_lexical(&mut self) {
    //     // let mut forms = self.take_field("http://www.w3.org/ns/lemon/ontolex#canonicalForm");
    //     // forms.extend(self.take_field("http://www.w3.org/ns/lemon/ontolex#otherForm"));
    //     self.parsed = Box::new(LexicalEntry {
    //         lime_language: self.take_at_most_one("http://www.w3.org/ns/lemon/lime#language"),
    //         language: self.take_at_most_one("http://purl.org/dc/terms/language"),
    //         // forms,
    //         // senses: self.take_field("http://www.w3.org/ns/lemon/ontolex#sense"),
    //     });
    // }
    // fn parse_form(&mut self) {
    //     self.parsed = Box::new(FormEntry {
    //         written: self.take_field("http://www.w3.org/ns/lemon/ontolex#writtenRep")
    //     });
    // }
    // fn parse_sense(&mut self) {
    //     self.parsed = Box::new(SenseEntry {});
    // }
    // fn parse_etymology(&mut self) {
    //     self.parsed = Box::new(EntymologyEntry {});
    // }
    // fn parse_translation(&mut self) {
    //     self.parsed = Box::new(TranslationEntry {});
    // }
    // fn parse_other(&mut self, types: Vec<Ustr>) {
    //     self.parsed = Box::new(OtherEntry { types });
    // }

    pub fn parse(&mut self) {
        let types = self.edges.get(&Ustr::from("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")).cloned().unwrap_or(vec![]);
        if types.contains(&Ustr::from("http://www.w3.org/ns/lemon/ontolex#Form")) {
            self.typ = Box::new(Form);
        } else if types.contains(&Ustr::from("http://www.w3.org/ns/lemon/ontolex#LexicalEntry")) {
            self.typ = Box::new(Lexical);
        } else if types.contains(&Ustr::from("http://kaiko.getalp.org/dbnary#Page")) {
            self.typ = Box::new(Page);
        } else if types.contains(&Ustr::from("http://etytree-virtuoso.wmflabs.org/dbnaryetymology#EtymologyEntry")) {
            self.typ = Box::new(Etymology);
        } else if types.contains(&Ustr::from("http://www.w3.org/ns/lemon/ontolex#LexicalSense")) {
            self.typ = Box::new(Sense);
        } else if types.contains(&Ustr::from("http://kaiko.getalp.org/dbnary#Translation")) {
            self.typ = Box::new(Translation);
        } else {
            self.typ = Box::new(Other);
        }
    }
}

impl<'a> NodeRef<'a, Page> {
    fn describes(&self) -> Vec<NodeRef<'a, Lexical>> {
        self.forward("http://kaiko.getalp.org/dbnary#describes")
    }
}

impl<'a> NodeRef<'a, Lexical> {
    fn describes_of(&self) -> Vec<NodeRef<'a, Page>> {
        self.reverse("http://kaiko.getalp.org/dbnary#describes")
    }
    fn canonical_form(&self) -> Vec<NodeRef<'a, Form>> {
        self.forward("http://www.w3.org/ns/lemon/ontolex#canonicalForm")
    }
    fn other_forms(&self) -> Vec<NodeRef<'a, Form>> {
        self.forward("http://www.w3.org/ns/lemon/ontolex#otherForm")
    }
}

impl<'a> NodeRef<'a, Form> {
    fn canonical_form_of(&self) -> Vec<NodeRef<'a, Lexical>> {
        self.reverse("http://www.w3.org/ns/lemon/ontolex#canonicalForm")
    }
    fn other_form_of(&self) -> Vec<NodeRef<'a, Lexical>> {
        self.reverse("http://www.w3.org/ns/lemon/ontolex#otherForm")
    }
    fn letters(&self) -> Vec<Vec<Letter>> {
        self.get("http://www.w3.org/ns/lemon/ontolex#writtenRep").iter().map(|x| get_alpha(&*x)).collect()
    }
}

impl<'a, T: 'static> NodeRef<'a, T> {
    pub fn forward<T2: 'static>(&self, name: &str) -> Vec<NodeRef<'a, T2>> {
        self.1.edges.get(&Ustr::from(name))
            .unwrap_or(&vec![])
            .iter()
            .map(|x| self.0.find(*x).unwrap())
            .collect()
    }
    pub fn reverse<T2: 'static>(&self, name: &str) -> Vec<NodeRef<'a, T2>> {
        self.1.in_edges.get(&Ustr::from(name))
            .unwrap_or(&vec![])
            .iter()
            .map(|x| self.0.find(*x).unwrap())
            .collect()
    }
    pub fn get(&self, name: &str) -> &[Ustr] {
        match self.1.edges.get(&Ustr::from(name)) {
            None => &[],
            Some(x) => x
        }
    }
}

impl Graph {
    fn parse(buffer: &str) -> Self {
        let mut graph = Graph { entries: default(), forms: Default::default() };
        let mut parser = TurtleParser::new(buffer.as_ref(), None);
        // for i in .. {
            parser.parse_all(&mut |t| {
                let subject = match t.subject {
                    Subject::NamedNode(node) => Ustr::from(node.iri),
                    Subject::BlankNode(x) => { Ustr::from(x.id) }
                    _ => panic!("{:?}", t.subject),
                };
                let entry = graph.entries.entry(subject).or_insert_with(|| NodeData {
                    typ: Box::new(()),
                    subject,
                    edges: Default::default(),
                    in_edges: default(),
                });
                let object = match t.object {
                    Term::Literal(literal) => {
                        match literal {
                            Literal::Simple { value } => Ustr::from(value),
                            Literal::LanguageTaggedString { value, language } => Ustr::from(value),
                            Literal::Typed { value, .. } => Ustr::from(value),
                        }
                    }
                    Term::NamedNode(x) => Ustr::from(x.iri),
                    Term::BlankNode(x) => Ustr::from(x.id),
                    _ => panic!("{:?}", t.object)
                };
                entry.edges.entry(Ustr::from(t.predicate.iri)).or_default().push(object);
                Ok(()) as Result<(), TurtleError>
            }).unwrap();
        // }
        let mut edges: Vec<_> = vec![];
        for entry in &mut graph.entries {
            entry.1.parse();
            edges.push((*entry.0, entry.1.edges.clone()));
        }
        for (from, edges) in edges {
            for (typ, to) in edges {
                for to in to {
                    if let Some(to) = graph.entries.get_mut(&to) {
                        to.in_edges.entry(typ).or_default().push(from);
                    }
                }
            }
        }
        let mut forms: HashMap<Vec<Letter>, Vec<_>> = HashMap::new();
        for form in graph.iter::<Form>() {
            for letters in form.letters() {
                forms.entry(letters).or_default().push(form.1.subject);
            }
        }
        graph.forms = forms;
        // let pe = graph.find::<Page>(Ustr::from("http://kaiko.getalp.org/dbnary/eng/dictionary")).unwrap();
        // let le = pe.describes().into_iter().next().unwrap();
        // let cf = le.canonical_form().into_iter().next().unwrap();
        println!("{:?}", graph.variants(get_alpha("dictionary")));
        println!("{:?}", graph.variants(get_alpha("dictionaries")));
        println!("{:?}", graph.variants(get_alpha("cat")));
        println!("{:?}", graph.variants(get_alpha("cats")));
        println!("{:?}", graph.variants(get_alpha("dog")));
        println!("{:?}", graph.variants(get_alpha("dogs")));
        println!("{:?}", graph.variants(get_alpha("abide")));
        // let le = graph.forward(pe, DESCRIBES).into_iter().next().unwrap();
        // let se = graph.forward(le, SENSE).into_iter().next().unwrap();

        // println!("{:?}", pe.1);
        // println!("{:#?}", graph.entries.get(&Ustr::from("http://kaiko.getalp.org/dbnary/eng/dictionary")));
        // println!("{:#?}", graph.entries.get(&Ustr::from("http://kaiko.getalp.org/dbnary/eng/dictionary__Noun__1")));
        graph
    }
    fn find<T: 'static>(&self, name: Ustr) -> Option<NodeRef<T>> {
        let node = self.entries.get(&name)?;
        (&*node.typ as &dyn Any).downcast_ref::<T>()?;
        Some(NodeRef(self, node, PhantomData))
    }
    fn iter<T: 'static>(&self) -> impl Iterator<Item=NodeRef<T>> {
        self.entries.iter().filter_map(|x| self.find(*x.0))
    }
    fn variants(&self, str: Vec<Letter>) -> Vec<Vec<Letter>> {
        let mut result = HashSet::<Vec<Letter>>::new();
        for form in self.forms.get(&str).unwrap() {
            let form = self.find::<Form>(*form).unwrap();
            for lexical in form.canonical_form_of().into_iter().chain(form.other_form_of().into_iter()) {
                for page in lexical.describes_of() {
                    for l2 in page.describes() {
                        for f2 in l2.canonical_form().into_iter().chain(l2.other_forms().into_iter()) {
                            for ls2 in f2.letters() {
                                result.insert(ls2);
                            }
                        }
                    }
                }
            }
        }
        result.into_iter().collect()
    }
    // fn get<T: 'static>(&self, node: Node<T>, field: Field<T>) -> &[Ustr] {
    //     match self.entries.get(&node.0).unwrap().edges.get(&Ustr::from(field.0)) {
    //         Some(x) => x,
    //         None => &[]
    //     }
    // }
    // fn forward<A: 'static, B: 'static>(&self, node: Node<A>, relation: Relation<A, B>) -> Vec<Node<B>> {
    //     self.entries
    //         .get(&node.0).unwrap()
    //         .edges
    //         .get(&Ustr::from(relation.0))
    //         .unwrap_or(&vec![]).into_iter()
    //         .map(|x| self.find(*x).unwrap())
    //         .collect()
    // }
    // fn reverse<A: 'static, B: 'static>(&self, node: Node<A>, relation: Relation<B, A>) -> Vec<Node<B>> {
    //     self.entries
    //         .get(&node.0).unwrap()
    //         .in_edges
    //         .get(&Ustr::from(relation.0))
    //         .unwrap_or(&vec![]).into_iter()
    //         .map(|x| self.find(*x).unwrap())
    //         .collect()
    // }
}

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let filename = PACKAGE_PATH.join("build/en_dbnary_ontolex.ttl");
    // let mut files = SimpleFiles::new();
    let buffer = fs::read_to_string(&filename).await?;
    // let file_id = files.add(format!("{:?}", filename), &buffer);
    let graph = Graph::parse(&buffer);
    // let mut triples: HashMap<Ustr, HashMap<Ustr, Vec<Ustr>>> = HashMap::new();
    // TurtleParser::new(buffer.as_ref(), None).parse_all(&mut |t| {
    //     let subject = match t.subject {
    //         Subject::NamedNode(node) => Ustr::from(node.iri),
    //         Subject::BlankNode(x) => { Ustr::from(x.id) }
    //         _ => panic!("{:?}", t.subject),
    //     };
    //     let object = match t.object {
    //         Term::Literal(Literal::LanguageTaggedString { value, .. }) => Ustr::from(value),
    //         Term::Literal(Literal::Simple { value }) => Ustr::from(value),
    //         Term::NamedNode(x) => Ustr::from(x.iri),
    //         Term::BlankNode(x) => Ustr::from(x.id),
    //         Term::Literal(Literal::Typed { value, datatype }) => Ustr::from(value),
    //         _ => panic!("{:?}", t.object)
    //     };
    //     triples.entry(subject).or_default().entry(Ustr::from(t.predicate.iri)).or_default().push(object);
    // match t.predicate.iri {
    //     "http://purl.org/olia/olia.owl#hasMood" => match &*object {
    //         "http://purl.org/olia/olia.owl#Participle" => {}
    //         x => panic!("{:?}", x),
    //     }
    //     "http://purl.org/olia/olia.owl#hasTense" => match &*object {
    //         "http://purl.org/olia/olia.owl#Past" => {}
    //         "http://purl.org/olia/olia.owl#Present" => {}
    //         x => panic!("{:?}", x),
    //     }
    //     "http://purl.org/olia/olia.owl#hasNumber" => match &*object {
    //         "http://purl.org/olia/olia.owl#Singular" => {}
    //         "http://purl.org/olia/olia.owl#Plural" => {}
    //         x => panic!("{:?}", x),
    //     }
    //     "http://www.w3.org/ns/lemon/ontolex#otherForm" => {
    //         panic!("{:?}", object)
    //     }
    //     "http://www.w3.org/ns/lemon/ontolex#writtenRep" => {}
    //     "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" => match &*object {
    //         "http://www.w3.org/ns/lemon/ontolex#Form" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Noun" => {}
    //         "http://www.w3.org/ns/lemon/ontolex#LexicalEntry" => {}
    //         "http://www.w3.org/ns/lemon/ontolex#Word" => {}
    //         "http://kaiko.getalp.org/dbnary#Page" => {}
    //         "http://www.w3.org/ns/lemon/ontolex#LexicalSense" => {}
    //         "http://kaiko.getalp.org/dbnary#Gloss" => {}
    //         "http://kaiko.getalp.org/dbnary#Translation" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Verb" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Adjective" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Adverb" => {}
    //         "http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Interjection" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#VerbPhrase" => {}
    //         "http://www.w3.org/ns/lemon/ontolex#MultiWordExpression" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Number" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Symbol" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#ProperNoun" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#NounPhrase" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Prefix" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#AdjectivePhrase" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Preposition" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Article" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Pronoun" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Determiner" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Conjunction" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Numeral" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Particle" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Suffix" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#PrepositionPhrase" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Postposition" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Infix" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#Affix" => {}
    //         x => panic!("{:?}", x),
    //     }
    //
    //     "http://purl.org/olia/olia.owl#hasDegree" => match &*object {
    //         "http://purl.org/olia/olia.owl#Superlative" => {}
    //         "http://purl.org/olia/olia.owl#Comparative" => {}
    //         x => panic!("{:?}", x),
    //     }
    //     "http://purl.org/olia/olia.owl#hasPerson" => match &*object {
    //         "http://purl.org/olia/olia.owl#Third" => {}
    //         x => panic!("{:?}", x),
    //     }
    //     "http://www.w3.org/2004/02/skos/core#note" => {}
    //     "http://purl.org/olia/olia.owl#hasGender" => match &*object {
    //         "http://purl.org/olia/olia.owl#Masculine" => {}
    //         "http://purl.org/olia/olia.owl#Feminine" => {}
    //         x => panic!("{:?}", x),
    //     }
    //     "http://www.w3.org/2000/01/rdf-schema#label" => {}
    //     "http://kaiko.getalp.org/dbnary#partOfSpeech" => match &*object {
    //         "Noun" => {}
    //         "Verb" => {}
    //         "Adjective" => {}
    //         "Adverb" => {}
    //         "Interjection" => {}
    //         "Number" => {}
    //         "Symbol" => {}
    //         "Proper noun" => {}
    //         "Prefix" => {}
    //         "Preposition" => {}
    //         "Article" => {}
    //         "Phrase" => {}
    //         "Pronoun" => {}
    //         "Determiner" => {}
    //         "Conjunction" => {}
    //         // x => panic!("{:?}", x),
    //         _ => {}
    //     }
    //     "http://purl.org/dc/terms/language" => match &*object {
    //         "http://lexvo.org/id/iso639-3/eng" => {}
    //         x => panic!("{:?}", x),
    //     }
    //     "http://www.lexinfo.net/ontology/2.0/lexinfo#partOfSpeech" => match &*object {
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#noun" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#verb" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#adjective" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#adverb" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#interjection" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#numeral" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#symbol" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#properNoun" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#prefix" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#preposition" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#article" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#phraseologicalUnit" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#pronoun" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#determiner" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#conjunction" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#suffix" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#particle" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#proverb" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#postposition" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#infix" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#affix" => {}
    //         "http://www.lexinfo.net/ontology/2.0/lexinfo#idiom" => {}
    //         x => println!("{:?}", x),
    //     }
    //     "http://www.w3.org/ns/lemon/lime#language" => match &*object {
    //         "en" => {}
    //         x => panic!("{:?}", x),
    //     }
    //     "http://www.w3.org/ns/lemon/ontolex#canonicalForm" => {
    //         // println!("{:?} {:?}", subject, object)
    //     }
    //     "http://www.w3.org/ns/lemon/ontolex#sense" => {
    //         // println!("{:?} {:?}", subject, object)
    //     }
    //     "http://www.w3.org/ns/lemon/ontolex#phoneticRep" => {
    //         //IPA
    //     }
    //     "http://kaiko.getalp.org/dbnary#describes" => {}
    //     "http://kaiko.getalp.org/dbnary#hypernym" => {}
    //     "http://kaiko.getalp.org/dbnary#hyponym" => {}
    //     "http://kaiko.getalp.org/dbnary#synonym" => {}
    //     "http://kaiko.getalp.org/dbnary#senseNumber" => {}
    //     "http://www.w3.org/1999/02/22-rdf-syntax-ns#value" => {}
    //     "http://www.w3.org/2004/02/skos/core#definition" => {}
    //     "http://www.w3.org/2004/02/skos/core#example" => {}
    //     "http://purl.org/dc/terms/bibliographicCitation" => {}
    //     "http://kaiko.getalp.org/dbnary#derivedFrom" => {
    //         //etymology
    //     }
    //     "http://kaiko.getalp.org/dbnary#rank" => {}
    //     "http://kaiko.getalp.org/dbnary#gloss" => {}
    //     "http://kaiko.getalp.org/dbnary#isTranslationOf" => {}
    //     "http://kaiko.getalp.org/dbnary#targetLanguage" => {}
    //     "http://kaiko.getalp.org/dbnary#writtenForm" => {
    //         //text
    //     }
    //     "http://kaiko.getalp.org/dbnary#usage" => {}
    //     "http://kaiko.getalp.org/dbnary#targetLanguageCode" => {}
    //     "http://kaiko.getalp.org/dbnary#antonym" => {}
    //     "http://www.w3.org/1999/02/22-rdf-syntax-ns#object" => {
    //         //link
    //     }
    //     "http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate" => {
    //         //link
    //     }
    //     "http://www.w3.org/1999/02/22-rdf-syntax-ns#subject" => {
    //         //link
    //     }
    //     "http://purl.org/olia/olia.owl#hasCountability" => match &*object {
    //         "http://purl.org/olia/olia.owl#Uncountable" => {}
    //         "http://purl.org/olia/olia.owl#Countable" => {}
    //         x => panic!("{:?}", x),
    //     }
    //     "http://kaiko.getalp.org/dbnary#holonym" => {}
    //     "http://kaiko.getalp.org/dbnary#meronym" => {}
    //     "http://www.w3.org/ns/lemon/vartrans#lexicalRel" => {}
    //     "http://kaiko.getalp.org/dbnary#troponym" => {}
    //     x => panic!("{:?}", x),
    // }
    //     Ok(()) as Result<(), TurtleError>
    // })?;
    Ok(())
}