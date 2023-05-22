use std::collections::HashSet;
use std::default::default;
use std::sync::Arc;
use brotli::interface::Command::Dict;
use crate::dict::FLAT_WORDS;
use crate::ontology::{Etymology, Form, Lexical, Ontology, ONTOLOGY, Page, Written};
use crate::turtle::TurtleIndex;
use crate::util::lazy_async::CloneError;

#[derive(Debug)]
pub struct ConflictSet {
    ontology: Arc<Ontology>,
    origins: HashSet<String>,
    writtens: HashSet<Written>,
    forms: HashSet<Form>,
    lexicals: HashSet<Lexical>,
    etyms: HashSet<Etymology>,
    relateds: HashSet<Etymology>,
    pages: HashSet<Page>,
    etyms_down: HashSet<Etymology>,
    pages_down: HashSet<Page>,
    lexicals_down: HashSet<Lexical>,
    writtens_down: HashSet<Written>,
    forms_down: HashSet<Form>,
    terminals: HashSet<String>,
}

impl ConflictSet {
    pub fn new(ontology: Arc<Ontology>) -> Self {
        ConflictSet {
            ontology,
            origins: default(),
            writtens: default(),
            forms: default(),
            lexicals: default(),
            etyms: default(),
            relateds: default(),
            pages: default(),
            etyms_down: default(),
            pages_down: default(),
            lexicals_down: default(),
            writtens_down: default(),
            forms_down: default(),
            terminals: default(),
        }
    }
    pub fn add_origin(&mut self, origin: String) {
        if self.origins.insert(origin.clone()) {
            if let Some(written) = self.ontology.find_written(&origin) {
                self.add_written(written);
            }
        }
    }
    pub fn add_written(&mut self, written: Written) {
        if self.writtens.insert(written) {
            // self.conflicts.insert(written.0);
            for form in self.ontology.written_rep_of(written) {
                self.add_form(form);
            }
        }
    }
    pub fn add_form(&mut self, form: Form) {
        if self.forms.insert(form) {
            // self.conflicts.insert(form.0);
            for x in self.ontology.canonical_form_of(form) {
                self.add_lexical(x);
            }
            for x in self.ontology.other_form_of(form) {
                self.add_lexical(x);
            }
        }
    }
    pub fn add_lexical(&mut self, lexical: Lexical) {
        if self.lexicals.insert(lexical) {
            self.add_lexical_down(lexical);
            // self.conflicts.insert(lexical.0);
            let (etyms, pages) = self.ontology.describes_of(lexical);
            for etym in etyms {
                self.add_etymology(etym);
            }
            for page in pages {
                self.add_page(page)
            }
        }
    }
    pub fn add_page(&mut self, page: Page) {
        if self.pages.insert(page) {
            self.add_page_down(page);
            for lexical in self.ontology.derived_from(page) {
                self.add_lexical(lexical);
            }
        }
    }
    pub fn add_etymology(&mut self, etym: Etymology) {
        if self.etyms.insert(etym) {
            self.add_etym_down(etym);
            for related in self.ontology.etym_related_to(etym) {
                self.add_related(related);
            }
        }
    }
    pub fn add_related(&mut self, related: Etymology) {
        if self.relateds.insert(related) {
            self.add_etym_down(related);
        }
    }
    pub fn add_etym_down(&mut self, etym: Etymology) {
        if self.etyms_down.insert(etym) {
            for lexical in self.ontology.describes_etym(etym) {
                self.add_lexical_down(lexical);
            }
        }
    }
    pub fn add_page_down(&mut self, page: Page) {
        if self.pages_down.insert(page) {
            for lexical in self.ontology.describes_page(page) {
                self.add_lexical_down(lexical);
            }
        }
    }
    pub fn add_lexical_down(&mut self, lexical: Lexical) {
        if self.lexicals_down.insert(lexical) {
            for x in self.ontology.canonical_form(lexical) {
                self.add_form_down(x);
            }
            for x in self.ontology.other_form(lexical) {
                self.add_form_down(x);
            }
        }
    }
    pub fn add_form_down(&mut self, form: Form) {
        if self.forms_down.insert(form) {
            for x in self.ontology.written_rep(form) {
                self.add_written_down(x)
            }
        }
    }
    pub fn add_written_down(&mut self, written: Written) {
        if self.writtens_down.insert(written) {
            self.add_terminal(self.ontology.graph.get_name(written.0));
        }
    }
    pub fn add_terminal(&mut self, terminal: &str) {
        self.terminals.insert(terminal.to_string());
    }
}

#[tokio::test]
async fn test_find_conflicts() -> anyhow::Result<()> {
    let ontology = ONTOLOGY.get().await.clone_error()?.clone();
    for word in ["definition"] {
        let mut conflicts = ConflictSet::new(ontology.clone());
        conflicts.add_origin(word.to_string());
        println!("{:#?}", conflicts);
    }
    Ok(())
}