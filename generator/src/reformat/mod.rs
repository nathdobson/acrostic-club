// Reformats a quote's text for its puzzle. The reformatted text is what the solved puzzle shows,
// and what stage 1 lays out as cells.
//
// The quote list's text reached this repository with damage, such as invisible characters and
// garbled punctuation. Each pass repairs one kind of damage, taking text and giving text, so the
// passes run one after another.

pub mod characters;

use characters::repair_characters;

// Runs each reformatting pass over a quote's text in turn.
pub fn reformat_quote(quote: &str) -> String {
    repair_characters(quote)
}
