// Times searches over the trie table: one search for word pairs, repeated, and the whole answer
// search on a sample of puzzles. Both tests read the dictionary and the tries from `build/`, the
// outputs of `global dict` and `global trie`.

use std::time::{Duration, Instant};

use acrostic_core::letter::{Letter, LetterSet};
use acrostic_generator::PACKAGE_PATH;
use acrostic_generator::puzzle::Puzzle;
use acrostic_generator::search::Search;
use acrostic_generator::trie_table::FLAT_TRIE_TABLE;
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
#[ignore = "needs the generator's outputs in build/"]
async fn ten_thousand_trie_table_searches_finish() -> anyhow::Result<()> {
    let first_letters = (
        Letter::new(b'e').expect("e should be an ASCII letter"),
        Letter::new(b'i').expect("i should be an ASCII letter"),
    );
    let letters = LetterSet::from_str("AEEEEEEEEEEEGIIKKKKLLLNNNOPPTTTWWWW");
    FLAT_TRIE_TABLE.get().await.clone_error()?;

    let start = Instant::now();
    for _ in 0..10000 {
        let mut found = vec![];
        FLAT_TRIE_TABLE
            .get()
            .await
            .clone_error()?
            .binary
            .get(&first_letters)
            .expect("the trie table should hold a trie for each pair of first letters")
            .search_smallest_subset(letters, 16, &mut found);
    }

    let elapsed = start.elapsed();
    println!("{elapsed:?}");
    Ok(())
}

// How many puzzles `answer_searches_finish_on_sample_puzzles` times, spread evenly through the
// puzzles that `list_feasible_puzzle_indices` returns.
const SAMPLE_SIZE: usize = 40;

// How many seeds to try on a puzzle before giving up on it. `add_answers` tries up to 1000, which
// would make the test far slower.
const SEED_LIMIT: u64 = 50;

// Lists the indices of the puzzles under `puzzles/`, in increasing order.
fn list_puzzle_indices() -> anyhow::Result<Vec<usize>> {
    let mut puzzle_indices = vec![];
    for entry in std::fs::read_dir(PACKAGE_PATH.join("puzzles"))? {
        let folder_name = entry?.file_name();
        let folder_name = folder_name
            .to_str()
            .expect("puzzle folder names should be text");
        if let Ok(puzzle_index) = folder_name.parse() {
            puzzle_indices.push(puzzle_index);
        }
    }
    puzzle_indices.sort();
    Ok(puzzle_indices)
}

// Reads the letters `add_answers` searches with: the quote's letters, and the source's letters
// in order.
async fn read_puzzle_letters(puzzle_index: usize) -> anyhow::Result<(LetterSet, Vec<Letter>)> {
    let puzzle = Puzzle::read(puzzle_index, "stage1.json").await?;
    let quote_letters = puzzle
        .quote_letters
        .as_ref()
        .expect("a stage 1 puzzle should have its quote's letters");
    let source_letters = puzzle
        .source_letters
        .as_ref()
        .expect("a stage 1 puzzle should have its source's letters");
    let quote: LetterSet = quote_letters.bytes().flat_map(Letter::new).collect();
    let source: Vec<Letter> = source_letters.bytes().flat_map(Letter::new).collect();
    Ok((quote, source))
}

// Lists the puzzles whose source letters all appear in the quote. The search fails the others
// before it looks for any words.
async fn list_feasible_puzzle_indices() -> anyhow::Result<Vec<usize>> {
    let mut feasible_indices = vec![];
    for puzzle_index in list_puzzle_indices()? {
        let (quote, source) = read_puzzle_letters(puzzle_index).await?;
        let source_letter_counts: LetterSet = source.into_iter().collect();
        if source_letter_counts.is_subset(quote) {
            feasible_indices.push(puzzle_index);
        }
    }
    Ok(feasible_indices)
}

// Runs `add_answers`'s answer search, single-threaded, on a sample of puzzles. Prints how long
// each puzzle took and the answers found, then the total time. Run this test on its own, so
// other tests don't compete for the processor:
//
// ```
// cargo test --release --test search answer_searches_finish_on_sample_puzzles -- --ignored \
//     --nocapture
// ```
#[tokio::test]
#[ignore = "needs the generator's outputs in build/"]
async fn answer_searches_finish_on_sample_puzzles() -> anyhow::Result<()> {
    let puzzle_indices = list_feasible_puzzle_indices().await?;
    let step = puzzle_indices.len() / SAMPLE_SIZE;
    let mut total_time = Duration::ZERO;
    for puzzle_index in puzzle_indices.into_iter().step_by(step).take(SAMPLE_SIZE) {
        let (quote, source) = read_puzzle_letters(puzzle_index).await?;
        let search = Search::new(quote, source).await?;

        let start = Instant::now();
        let mut seeds_tried = 0;
        let mut solution = None;
        for seed in 0..SEED_LIMIT {
            seeds_tried += 1;
            // `add_answers` numbers its seeds the same way.
            solution = search.solve(puzzle_index as u64 * 1000 + seed);
            if solution.is_some() {
                break;
            }
        }
        let elapsed = start.elapsed();
        total_time += elapsed;

        let answers = match &solution {
            Some(solution) => search.format(solution),
            None => "no solution".to_owned(),
        };
        println!("puzzle {puzzle_index}: {seeds_tried} seeds, {elapsed:?}: {answers}");
    }
    println!("total: {total_time:?}");
    Ok(())
}
