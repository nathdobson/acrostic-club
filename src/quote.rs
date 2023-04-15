use std::{fs, io};

use crate::PACKAGE_PATH;
use serde::Serialize;
use serde::Deserialize;
#[derive(Debug, Serialize, Deserialize)]
pub struct Quote {
    pub quote: String,
    pub source: String,
    pub topics: Vec<String>,
}

impl Quote {
    pub fn get() -> io::Result<Vec<Quote>> {
        Ok(serde_json::from_str(&fs::read_to_string(PACKAGE_PATH.join("data/quotes.json"))?)?)
    }
}
