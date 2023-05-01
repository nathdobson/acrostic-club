#![feature(option_get_or_insert_default)]
#![feature(future_join)]
#![allow(unused_imports)]
#![deny(unused_must_use)]
#![allow(unused_variables, dead_code)]
#![allow(unused_mut)]

pub mod watch;
mod key_val;

use std::collections::HashMap;
use std::future::join;
use std::{io, thread};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::mpsc::RecvError;
use std::time::Duration;
use futures_util::{FutureExt, SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use parking_lot::Mutex;
use warp::Filter;
use warp::ws::{Message, WebSocket};
use serde::Serialize;
use serde::Deserialize;
use serde_json::Value;
use tokio::try_join;
use crate::key_val::KeyVal;
use crate::watch::Watchable;
use clap::Parser;
use log::LevelFilter;

#[derive(Default)]
pub struct RoomState {
    sender: Watchable<KeyVal<String, Value>>,
}

#[derive(Default)]
pub struct Room(Mutex<RoomState>);

pub struct RoomSet(parking_lot::Mutex<HashMap<String, Arc<Room>>>);

type Result<T> = anyhow::Result<T>;

impl Room {
    pub async fn run_socket(&self, mut tx: SplitSink<WebSocket, Message>, mut rx: SplitStream<WebSocket>) -> Result<()> {
        let watcher = self.0.lock().sender.watch();
        let down = async move {
            while let std::result::Result::<_, RecvError>::Ok(next) = watcher.recv().await {
                let payload = serde_json::to_string(&next).map_err(|x| io::Error::from(x))?;
                tx.send(Message::text(payload)).await?;
            }
            Result::Ok(())
        };
        let up = async move {
            while let Some(next) = rx.next().await.transpose()? {
                let message: KeyVal<String, Value> = serde_json::from_slice(next.as_bytes())?;
                self.0.lock().sender.modify(|x| x.merge_from(&message));
            }
            Result::Ok(())
        };
        try_join!(down, up)?;
        Ok(())
    }
}

impl RoomSet {
    pub fn new() -> Arc<RoomSet> {
        Arc::new(RoomSet(parking_lot::Mutex::new(HashMap::new())))
    }
    pub async fn run_socket(&self, name: &str, mut tx: SplitSink<WebSocket, Message>, mut rx: SplitStream<WebSocket>) -> Result<()> {
        let room = self.0.lock().entry(name.to_string()).or_default().clone();
        room.run_socket(tx, rx).await?;
        Ok(())
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, required = true)]
    bind_http: Option<SocketAddr>,

    #[arg(long, required = true)]
    bind_https: Option<SocketAddr>,

    #[arg(short, long, required = true)]
    cert: String,

    #[arg(short, long, required = true)]
    key: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    simple_logging::log_to_file("server.log", LevelFilter::Trace)?;
    let args: Args = Args::parse();
    let room_set = RoomSet::new();
    let routes =
        warp::path("room")
            .and(warp::path::param())
            .and(warp::ws())
            .map(move |name: String, ws: warp::ws::Ws| {
                println!("Handling");
                let room_set = room_set.clone();
                ws.on_upgrade(|websocket| async move {
                    println!("Upgraded");
                    let (tx, rx) = websocket.split();
                    if let Err(e) = room_set.run_socket(&name, tx, rx).await {
                        eprintln!("Websocket error: {:?}", e);
                    }
                })
            }).or(
            warp::path("test").map(|| {
                "Hello"
            })
        );
    let http = async {
        if let Some(bind_http) = args.bind_http {
            warp::serve(routes.clone()).run(bind_http).await;
        }
    };
    let https = async {
        if let Some(bind_https) = args.bind_https {
            warp::serve(routes.clone())
                .tls()
                .cert_path(args.cert)
                .key_path(args.key)
                .run(bind_https).await;
        }
    };
    join!(http, https).await;
    Ok(())
}