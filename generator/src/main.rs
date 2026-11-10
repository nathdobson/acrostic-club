#![deny(unused_must_use)]

use std::collections::BTreeSet;
use std::ops::Deref;
use std::{io, mem};

use futures::{StreamExt, stream};
use tikv_jemallocator::Jemalloc;

use acrostic_generator::add_letters::add_letters;
use acrostic_generator::clues::{ClueClient, add_chat};
use acrostic_generator::dict::build_dict;
use acrostic_generator::quote::{add_quote, build_quotes};
use acrostic_generator::search::add_answers;
use acrostic_generator::site::build_site;
use acrostic_generator::trie::build_trie;
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
            Some("trie") => build_trie().await?,
            Some("site") => build_site().await?,
            Some("turtle") => build_ontolex_turtle().await?,
            x => panic!("Unknown global target {:?}", x),
        },
        Some("puzzle") => {
            let target = args.next().unwrap();
            let mut puzzles = BTreeSet::new();
            for puzzle in args {
                if let Some((start, end)) = puzzle.split_once("-") {
                    for x in start.parse().unwrap()..=end.parse().unwrap() {
                        puzzles.insert(x);
                    }
                } else {
                    puzzles.insert(puzzle.parse().unwrap());
                }
            }
            let client = ClueClient::new(cleanup).await?;
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
                                "chat" => add_chat(puzzle, &client).await?,
                                x => panic!("Unknown puzzle target {}", x),
                            }
                            anyhow::Ok(())
                        }
                        .await;
                        if let Err(e) = e {
                            if e.downcast_ref::<io::Error>()
                                .map_or(true, |x| x.kind() != io::ErrorKind::NotFound)
                            {
                                eprintln!("puzzle={} {}", puzzle, e);
                                return Some(e);
                            }
                        } else {
                            eprintln!("puzzle={} done", puzzle);
                        }
                        None
                    }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<Option<anyhow::Error>>>()
                .await;
            mem::drop(client);
            eprintln!("{:?}", errors);
        }
        x => panic!("Unknown root command {:?}", x),
    }
    Ok(())
}
