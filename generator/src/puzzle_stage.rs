use anyhow::Context;

use crate::add_letters::add_letters;
use crate::clues::{ClueClient, add_chat, preview_chat};
use crate::command_line::PuzzleStage;
use crate::maintenance::rebuild_cells::rebuild_cells_in_files;
use crate::quote::{QuoteCatalog, add_quote};
use crate::search::add_answers;
use crate::trie_table::{FLAT_TRIE_TABLE, FlatTrieTable};
use crate::util::interrupt::CleanupSender;
use crate::util::lazy_async::CloneError;

// A puzzle stage holding the inputs it shares across puzzles. Loading them before the first puzzle
// reports a missing input once, as an error. Inside `main`'s loop over puzzles, a missing file
// instead means a puzzle that doesn't exist, which the loop skips without printing anything.
pub enum LoadedPuzzleStage {
    Quote(QuoteCatalog),
    Letters,
    Answers(&'static FlatTrieTable),
    Chat {
        client: ClueClient,
        // Whether to show the clues without saving them.
        is_dry_run: bool,
    },
    RebuildCells,
}

impl LoadedPuzzleStage {
    // Loads the inputs that `stage` shares across puzzles. Only the chat stage reads `cleanup`,
    // `model`, and `is_dry_run`.
    pub async fn load(
        stage: PuzzleStage,
        cleanup: CleanupSender,
        model: &str,
        is_dry_run: bool,
    ) -> anyhow::Result<Self> {
        Ok(match stage {
            // Repairing the quotes and finding the duplicates among them takes a pass over the
            // whole quote list.
            PuzzleStage::Quote => Self::Quote(QuoteCatalog::load().await?),
            PuzzleStage::Letters => Self::Letters,
            // The answers stage searches tries built from the word list.
            PuzzleStage::Answers => {
                let trie_table =
                    FLAT_TRIE_TABLE.get().await.clone_error_static().context(
                        "The answers stage needs the word list that `global dict` builds",
                    )?;
                Self::Answers(trie_table)
            }
            // Building the clue client loads the ontology, the lemma list, and the clue database.
            PuzzleStage::Chat => Self::Chat {
                client: ClueClient::new(cleanup, model).await?,
                is_dry_run,
            },
            PuzzleStage::RebuildCells => Self::RebuildCells,
        })
    }

    // Runs the stage on the puzzle numbered `puzzle_number`.
    pub async fn run(&self, puzzle_number: usize) -> anyhow::Result<()> {
        match self {
            Self::Quote(quote_catalog) => add_quote(puzzle_number, quote_catalog).await?,
            Self::Letters => add_letters(puzzle_number).await?,
            Self::Answers(trie_table) => add_answers(puzzle_number, trie_table).await?,
            Self::Chat {
                client,
                is_dry_run: true,
            } => preview_chat(puzzle_number, client).await?,
            Self::Chat {
                client,
                is_dry_run: false,
            } => add_chat(puzzle_number, client).await?,
            Self::RebuildCells => rebuild_cells_in_files(puzzle_number).await?,
        }
        Ok(())
    }
}
