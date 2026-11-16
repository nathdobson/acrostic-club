#![deny(unused_must_use)]
#![allow(unused_variables, dead_code)]
#![allow(unused_mut)]

pub mod key_val;
pub mod watch;

use crate::key_val::KeyVal;
use crate::watch::Watchable;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::mpsc::RecvError;
use tokio::try_join;
use warp::ws::{Message, WebSocket};

#[derive(Default)]
pub struct RoomState {
    sender: Watchable<KeyVal<String, Value>>,
}

#[derive(Default)]
pub struct Room(Mutex<RoomState>);

pub struct RoomSet(parking_lot::Mutex<HashMap<String, Arc<Room>>>);

type Result<T> = anyhow::Result<T>;

impl Room {
    pub async fn run_socket(
        &self,
        mut tx: SplitSink<WebSocket, Message>,
        mut rx: SplitStream<WebSocket>,
    ) -> Result<()> {
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
    pub async fn run_socket(
        &self,
        name: &str,
        mut tx: SplitSink<WebSocket, Message>,
        mut rx: SplitStream<WebSocket>,
    ) -> Result<()> {
        let room = self.0.lock().entry(name.to_string()).or_default().clone();
        room.run_socket(tx, rx).await?;
        Ok(())
    }
}
