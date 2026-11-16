#![feature(future_join)]

use acrostic_server::RoomSet;
use clap::Parser;
use futures_util::StreamExt;
use log::LevelFilter;
use std::future::join;
use std::io;
use std::net::SocketAddr;
use warp::Filter;

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
    let routes = warp::path("room")
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
        })
        .or(warp::path("test").map(|| "Hello"));
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
                .run(bind_https)
                .await;
        }
    };
    join!(http, https).await;
    Ok(())
}
