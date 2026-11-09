use crate::add_letters::quote_to_cells;
use crate::puzzle::Puzzle;
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;

// The position and letter of each letter cell, skipping spaces, hyphens, and digits.
fn letter_positions(cells: &str) -> Vec<(usize, char)> {
    cells
        .chars()
        .enumerate()
        .filter(|(_, cell)| cell.is_ascii_uppercase())
        .collect()
}

// Rebuilds a puzzle's cells with the current `quote_to_cells`, moving each clue's indices to
// follow the letters they point at.
//
// Spaces, hyphens, and digits may appear, vanish, or move, but the letters must not change:
// the clues' answers are spelled from them, so the rebuild fails if they do. A failed rebuild
// leaves the puzzle as it was.
pub fn rebuild_cells(puzzle: &mut Puzzle) -> io::Result<()> {
    let invalid_data = |message: String| io::Error::new(ErrorKind::InvalidData, message);
    let Some(old_cells) = puzzle.quote_letters.as_deref() else {
        return Err(invalid_data("the puzzle has no quote letters".to_string()));
    };
    let new_cells = quote_to_cells(&puzzle.quote);

    let old_letters = letter_positions(old_cells);
    let new_letters = letter_positions(&new_cells);
    let old_spelling: String = old_letters.iter().map(|&(_, letter)| letter).collect();
    let new_spelling: String = new_letters.iter().map(|&(_, letter)| letter).collect();
    if old_spelling != new_spelling {
        let message = format!("letters changed from {old_cells:?} to {new_cells:?}");
        return Err(invalid_data(message));
    }

    // Each letter's old position, mapped to its new one.
    let new_position_by_old: HashMap<usize, usize> = old_letters
        .iter()
        .zip(&new_letters)
        .map(|(&(old_position, _), &(new_position, _))| (old_position, new_position))
        .collect();

    // Check every index before moving any, so that a failure leaves the puzzle as it was.
    for clue in puzzle.clues.iter().flatten() {
        for index in &clue.indices {
            if !new_position_by_old.contains_key(index) {
                let answer = &clue.answer;
                let message =
                    format!("answer {answer:?} points at non-letter cell {index} of {old_cells:?}");
                return Err(invalid_data(message));
            }
        }
    }
    for clue in puzzle.clues.iter_mut().flatten() {
        for index in clue.indices.iter_mut() {
            let old_position = *index;
            *index = new_position_by_old[&old_position];
        }
    }
    puzzle.quote_letters = Some(new_cells);
    Ok(())
}

// Rebuilds the cells in each stage file a committed puzzle has, as `rebuild_cells` describes.
//
// A puzzle with no stage1.json has no cells to rebuild, so the function returns the `NotFound`
// error from reading it. The stage targets, such as `add_letters`, report a missing input file the
// same way, and `main` skips any puzzle that returns `NotFound` without printing anything. That
// way, a range of puzzle numbers may include numbers that have no puzzle.
pub async fn rebuild_cells_in_files(puzzle_index: usize) -> io::Result<()> {
    for stage in ["stage1.json", "stage2.json", "stage3.json"] {
        let mut puzzle = match Puzzle::read(puzzle_index, stage).await {
            Ok(puzzle) => puzzle,
            // Any error reading stage1.json goes back to the caller, `NotFound` included, as the
            // function's comment describes.
            Err(error) if stage == "stage1.json" => return Err(error),
            // Each stage is built from the one before it, so the puzzle hasn't reached this stage
            // or any later one, and every stage it has is already rebuilt.
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if let Err(error) = rebuild_cells(&mut puzzle) {
            let message = format!("puzzle {puzzle_index} {stage}: {error}");
            return Err(io::Error::new(error.kind(), message));
        }
        puzzle.write(puzzle_index, stage).await?;
    }
    Ok(())
}
