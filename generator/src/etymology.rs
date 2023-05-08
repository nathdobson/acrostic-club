use arrayvec::ArrayString;

#[derive(Debug)]
pub struct EtymologyEntry {
    term_id: String,
    lang: String,
    term: String,
    reltype: String,
    related_term_id: String,
    related_lang: String,
    related_term: String,
    position: String,
    group_tag: String,
    parent_tag: String,
    parent_position: String,
}

