// Reformats a quote's text for its puzzle. The reformatted text is what the solved puzzle shows,
// and what stage 1 lays out as cells.
//
// The quote list's text reached this repository with damage, such as invisible characters, garbled
// punctuation, and lost spaces. Each pass repairs one kind of damage, taking text and giving text,
// so the passes run one after another: `RepairedQuote` repairs the characters, and then restores
// the lost spaces.

pub mod characters;
pub mod lost_spaces;
