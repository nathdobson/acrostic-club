use std::{io, mem};

use anyhow::Context;
use futures::{StreamExt, stream};
use tikv_jemallocator::Jemalloc;

use acrostic_generator::cluebank::{ClueEdit, edit_clues, import_clues};
use acrostic_generator::command_line::{
    EditClueArguments, PuzzleArguments, PuzzleCommand, parse_edit_clue_arguments,
    parse_import_clues_arguments, parse_puzzle_arguments, parse_puzzle_command,
};
use acrostic_generator::dict::build_dict;
use acrostic_generator::joined_words::fill_joined_word_bank;
use acrostic_generator::llm::{DEFAULT_MODEL_NAME, new_client};
use acrostic_generator::puzzle_stage::LoadedPuzzleStage;
use acrostic_generator::quote::build_quotes;
use acrostic_generator::site::build_site;
use acrostic_generator::turtle::db::build_ontolex_turtle;
use acrostic_generator::util::interrupt::{CleanupSender, run_with_interrupts};

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
            // `global joined-words --model <name> [<puzzles>]` adds the model's splits to the
            // joined-word bank. The model reads the quotes of the puzzles given, or every quote
            // that fits a puzzle when none are given.
            Some("joined-words") => {
                let PuzzleArguments {
                    puzzle_numbers,
                    model,
                    is_dry_run,
                } = parse_puzzle_arguments(args)?;
                // The command always writes the bank, so it refuses `--dry-run` rather than write
                // anyway.
                if is_dry_run {
                    anyhow::bail!("`global joined-words` has no dry run");
                }
                let model = model.context("`global joined-words` needs `--model <name>`")?;
                let puzzle_numbers = (!puzzle_numbers.is_empty()).then_some(&puzzle_numbers);
                let client = new_client(cleanup).await?;
                fill_joined_word_bank(&model, puzzle_numbers, &*client).await?
            }
            target => panic!("Unknown global target {target:?}"),
        },
        Some("puzzle") => {
            let PuzzleCommand {
                stage,
                arguments:
                    PuzzleArguments {
                        puzzle_numbers: puzzles,
                        model,
                        is_dry_run,
                    },
            } = parse_puzzle_command(args)?;
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
