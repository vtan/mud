mod event_writer;
mod game_logic;
mod game_state;
mod id;
mod line;
mod server_actor;
mod server_websocket;

use std::{fs, net::SocketAddr};

use game_state::Room;
use server_websocket::{handle_connection, ConnectQuery};
use tokio::sync::mpsc;
use warp::Filter;

#[tokio::main]
async fn main() {
    env_logger::init();

    let rooms = {
        let list: Vec<Room> =
            serde_yaml::from_str(&fs::read_to_string("data/rooms.yaml").unwrap()).unwrap();
        list.into_iter().map(|room| (room.id, room)).collect()
    };

    let socket_address = std::env::var("MUD_ADDR")
        .ok()
        .and_then(|str| str.parse::<SocketAddr>().ok())
        .unwrap_or(([127, 0, 0, 1], 8081).into());

    let (actor_sender, actor_receiver) = mpsc::channel::<server_actor::Message>(4096);
    tokio::spawn(async { server_actor::run(actor_receiver, rooms).await });

    let routes = warp::path!("api" / "ws")
        .and(warp::query::<ConnectQuery>())
        .and(warp::ws())
        .map(move |query: ConnectQuery, ws: warp::ws::Ws| {
            let message_sender = actor_sender.clone();
            ws.on_upgrade(|websocket| handle_connection(websocket, query, message_sender))
        });

    warp::serve(routes).run(socket_address).await;
}
