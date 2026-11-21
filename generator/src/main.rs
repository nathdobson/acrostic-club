use std::ops::Deref;
use std::{io, mem};

use futures::{StreamExt, stream};
use tikv_jemallocator::Jemalloc;

use acrostic_generator::add_letters::add_letters;
use acrostic_generator::cluebank::{ClueEdit, edit_clues};
use acrostic_generator::clues::{ClueClient, add_chat, preview_chat};
use acrostic_generator::command_line::{
    EditClueArguments, PuzzleArguments, parse_edit_clue_arguments, parse_puzzle_arguments,
};
use acrostic_generator::dict::build_dict;
use acrostic_generator::llm::DEFAULT_MODEL_NAME;
use acrostic_generator::maintenance::rebuild_cells::rebuild_cells_in_files;
use acrostic_generator::quote::{add_quote, build_quotes};
use acrostic_generator::search::add_answers;
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
    args.next().unwrap();
    match args.next().as_deref() {
        Some("global") => match args.next().as_deref() {
            Some("quotes") => build_quotes().await?,
            Some("dict") => build_dict().await?,
            Some("site") => build_site().await?,
            Some("turtle") => build_ontolex_turtle().await?,
            // Records a person's edit of one clue's text, along with the person's satisfying
            // review of it, renaming the clue's tag in the puzzles too when the tag changes. Only
            // people run this command, since it credits every edit to a person: a model's edits go
            // through `edit_clues` with the model as their source.
            Some("edit-clue-as-person") => {
                let EditClueArguments {
                    answer,
                    tag,
                    new_text,
                } = parse_edit_clue_arguments(args)?;
                edit_clues(&[ClueEdit::by_person(&answer, &tag, &new_text)]).await?
            }
            target => panic!("Unknown global target {target:?}"),
        },
        Some("puzzle") => {
            let target = args.next().unwrap();
            let PuzzleArguments {
                puzzle_numbers: puzzles,
                model,
                is_dry_run,
            } = parse_puzzle_arguments(args)?;
            // Only the chat stage asks a language model, so `--model` does nothing elsewhere.
            if model.is_some() && target != "chat" {
                anyhow::bail!(
                    "`puzzle {target}` asks no language model, so `--model` has nothing to choose"
                );
            }
            // Only the chat stage can run without saving its work, so any other target refuses
            // `--dry-run` rather than save anyway.
            if is_dry_run && target != "chat" {
                anyhow::bail!("`puzzle {target}` has no dry run; only `puzzle chat` does");
            }
            let model = model.as_deref().unwrap_or(DEFAULT_MODEL_NAME);
            // Building the clue client loads the ontology, the lemma list, and the clue database,
            // which only the chat stage reads.
            let client = if target == "chat" {
                Some(ClueClient::new(cleanup, model).await?)
            } else {
                None
            };
            let concurrency = match target.deref() {
                "chat" => 1,
                _ => 1,
            };
            let errors = stream::iter(puzzles.into_iter())
                .map(|puzzle| {
                    let target = &target;
                    let client = &client;
                    async move {
                        let e: anyhow::Result<()> = async {
                            match target.deref() {
                                "quote" => add_quote(puzzle).await?,
                                "letters" => add_letters(puzzle).await?,
                                "answers" => add_answers(puzzle).await?,
                                "chat" => {
                                    let client = client
                                        .as_ref()
                                        .expect("the chat stage should have built a clue client");
                                    if is_dry_run {
                                        preview_chat(puzzle, client).await?
                                    } else {
                                        add_chat(puzzle, client).await?
                                    }
                                }
                                "rebuild-cells" => rebuild_cells_in_files(puzzle).await?,
                                _ => panic!("Unknown puzzle target {target}"),
                            }
                            anyhow::Ok(())
                        }
                        .await;
                        if let Err(e) = e {
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
                .buffer_unordered(concurrency)
                .collect::<Vec<Option<anyhow::Error>>>()
                .await;
            mem::drop(client);
            eprintln!("{errors:?}");
        }
        command => panic!("Unknown root command {command:?}"),
    }
    Ok(())
}
