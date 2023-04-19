#![feature(option_get_or_insert_default)]
#![allow(unused_imports)]
#![deny(unused_must_use)]
#![allow(unused_variables, dead_code)]
#![allow(unused_mut)]

pub mod watch;
mod key_val;

use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::mpsc::RecvError;
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
//
// async fn run_socket(mut tx: SplitSink<WebSocket, Message>, mut rx: SplitStream<WebSocket>) -> Result<(), warp::Error> {
//     while let Some(message) = rx.next().await.transpose()? {
//         tx.send(message).await?;
//     }
//     Ok(())
// }

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

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let room_set = RoomSet::new();

    let routes =
        warp::path("room")
            .and(warp::path::param())
            .and(warp::ws())
            .map(move |name: String, ws: warp::ws::Ws| {
                println!("Handling");
                let room_set = room_set.clone();
                ws.on_upgrade(|websocket| async move {
                    let (tx, rx) = websocket.split();
                    if let Err(e) = room_set.run_socket(&name, tx, rx).await {
                        eprintln!("websocket error: {:?}", e);
                    }
                })
            });

    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}