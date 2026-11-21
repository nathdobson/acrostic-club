// Reformats a quote's text for its puzzle. The reformatted text is what the solved puzzle shows,
// and what stage 1 lays out as cells.
//
// The quote list's text reached this repository with damage, such as invisible characters, garbled
// punctuation, and lost spaces. Each pass repairs one kind of damage, taking text and giving text,
// so the passes run one after another.

pub mod characters;
pub mod lost_spaces;

use std::io;

use characters::repair_characters;
use lost_spaces::restore_lost_spaces;

// Repairs a quote's characters, then restores its lost spaces. Fails on a quote that no puzzle can
// show, such as one holding `☥`.
pub fn reformat_quote(quote: &str) -> io::Result<String> {
    let repaired = repair_characters(quote);
    restore_lost_spaces(&repaired)
}
