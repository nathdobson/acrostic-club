use std::io;
use rand::seq::SliceRandom;
use rand::thread_rng;

use acrostic::puzzle::Puzzle;
use acrostic::quote::Quote;

#[tokio::main]
async fn main() -> io::Result<()> {

    let mut quotes = Quote::get()?;
    quotes.shuffle(&mut thread_rng());
    let mut selected = None;
    for quote in quotes {
        if quote.source.len() > 22
            && quote.source.len() <= 24
            && quote.quote.len() > 180
            && quote.quote.len() < 200
        {
            selected = Some(quote);
        }
    }
    let selected = selected.unwrap();
    let puzzle = Puzzle {
        quote: selected.quote,
        quote_letters: None,
        source: selected.source,
        source_letters: None,
        clues: None,
        chat: None
    };
    puzzle.write("stage0.json")?;
    Ok(())
}
