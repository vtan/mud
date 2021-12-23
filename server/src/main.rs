mod game_logic;
mod game_state;
mod id;
mod line;
mod server_actor;
mod server_websocket;

use std::fs;

use game_state::Room;
use server_websocket::{handle_connection, ConnectQuery};
use tokio::sync::mpsc;
use warp::Filter;

#[tokio::main]
async fn main() {
    env_logger::init();

    let rooms = {
        let list: Vec<Room> =
            serde_json::from_str(&fs::read_to_string("data/rooms.json").unwrap()).unwrap();
        list.into_iter().map(|room| (room.id, room)).collect()
    };

    let port = std::env::var("MUD_PORT")
        .ok()
        .and_then(|str| str.parse::<u16>().ok())
        .unwrap_or(8081);

    let (actor_sender, actor_receiver) = mpsc::channel::<server_actor::Message>(4096);
    tokio::spawn(async { server_actor::run(actor_receiver, rooms).await });

    let routes = warp::path!("api" / "ws")
        .and(warp::query::<ConnectQuery>())
        .and(warp::ws())
        .map(move |query: ConnectQuery, ws: warp::ws::Ws| {
            let message_sender = actor_sender.clone();
            ws.on_upgrade(|websocket| handle_connection(websocket, query, message_sender))
        });

    warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}
