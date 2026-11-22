use std::{io, mem};

use anyhow::Context;
use futures::{StreamExt, stream};
use tikv_jemallocator::Jemalloc;

use acrostic_generator::add_letters::add_letters;
use acrostic_generator::cluebank::{ClueEdit, edit_clues, import_clues};
use acrostic_generator::clues::{ClueClient, add_chat, preview_chat};
use acrostic_generator::command_line::{
    EditClueArguments, PuzzleArguments, PuzzleStage, parse_edit_clue_arguments,
    parse_import_clues_arguments, parse_puzzle_arguments, parse_puzzle_stage,
};
use acrostic_generator::dict::build_dict;
use acrostic_generator::llm::DEFAULT_MODEL_NAME;
use acrostic_generator::maintenance::rebuild_cells::rebuild_cells_in_files;
use acrostic_generator::quote::{QuoteCatalog, add_quote, build_quotes};
use acrostic_generator::search::add_answers;
use acrostic_generator::site::build_site;
use acrostic_generator::trie_table::{FLAT_TRIE_TABLE, FlatTrieTable};
use acrostic_generator::turtle::db::build_ontolex_turtle;
use acrostic_generator::util::interrupt::{CleanupSender, run_with_interrupts};
use acrostic_generator::util::lazy_async::CloneError;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(run_with_interrupts(main_impl).await?)
}

async fn main_impl(cleanup: CleanupSender) -> anyhow::Result<()> {
    let mut args = std::env::args().collect::<Vec<_>>().into_iter();
    args.next()
        .expect("the arguments should start with the program's path");
    match args.next().as_deref() {
        Some("global") => match args.next().as_deref() {
            Some("quotes") => build_quotes().await?,
            Some("dict") => build_dict().await?,
            Some("site") => build_site().await?,
            Some("turtle") => build_ontolex_turtle().await?,
            // Records a person's edit of one clue's text, along with the person's satisfying review
            // of it, renaming the clue's tag in the puzzles too when the tag changes. Only people
            // run this command, since it credits every edit to a person: a model's edits go through
            // `edit_clues` with the model as their source.
            Some("edit-clue-as-person") => {
                let EditClueArguments {
                    answer,
                    tag,
                    new_text,
                } = parse_edit_clue_arguments(args)?;
                edit_clues(&[ClueEdit::by_person(&answer, &tag, &new_text)]).await?
            }
            // Adds clues written elsewhere, such as by a clue-writing experiment, to the bank.
            Some("import-clues") => {
                let import_path = parse_import_clues_arguments(args)?;
                import_clues(&import_path).await?
            }
            target => panic!("Unknown global target {target:?}"),
        },
        Some("puzzle") => {
            let target = args
                .next()
                .context("`puzzle` needs a target, such as `answers` or `chat`")?;
            let stage = parse_puzzle_stage(&target)?;
            let PuzzleArguments {
                puzzle_numbers: puzzles,
                model,
                is_dry_run,
            } = parse_puzzle_arguments(args)?;
            // Only the chat stage asks a language model, so `--model` does nothing elsewhere.
            if model.is_some() && stage != PuzzleStage::Chat {
                anyhow::bail!(
                    "`puzzle {target}` asks no language model, so `--model` has nothing to choose"
                );
            }
            // Only the chat stage can run without saving its work, so any other target refuses
            // `--dry-run` rather than save anyway.
            if is_dry_run && stage != PuzzleStage::Chat {
                anyhow::bail!("`puzzle {target}` has no dry run; only `puzzle chat` does");
            }
            let model = model.as_deref().unwrap_or(DEFAULT_MODEL_NAME);
            let stage = LoadedPuzzleStage::load(stage, cleanup, model, is_dry_run).await?;
            let errors = stream::iter(puzzles.into_iter())
                .map(|puzzle| {
                    let stage = &stage;
                    async move {
                        if let Err(e) = stage.run(puzzle).await {
                            if e.downcast_ref::<io::Error>()
                                .map_or(true, |x| x.kind() != io::ErrorKind::NotFound)
                            {
                                // The alternate form prints the error's whole chain of causes,
                                // where the plain form prints only the outermost context.
                                eprintln!("puzzle={puzzle} {e:#}");
                                return Some(e);
                            }
                        } else {
                            eprintln!("puzzle={puzzle} done");
                        }
                        None
                    }
                })
                // One puzzle at a time, which `add_chat` relies on so that its writes to the clue
                // bank never race.
                .buffer_unordered(1)
                .collect::<Vec<Option<anyhow::Error>>>()
                .await;
            mem::drop(stage);
            eprintln!("{errors:?}");
        }
        command => panic!("Unknown root command {command:?}"),
    }
    Ok(())
}

// A puzzle stage holding the inputs it shares across puzzles. Loading them before the first puzzle
// reports a missing input once, as an error. Inside the loop over puzzles, a missing file instead
// means a puzzle that doesn't exist, which the loop skips without printing anything.
enum LoadedPuzzleStage {
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
    async fn load(
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
    async fn run(&self, puzzle_number: usize) -> anyhow::Result<()> {
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
