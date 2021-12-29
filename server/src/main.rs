mod event_writer;
mod game_alias;
mod game_chat;
mod game_combat;
mod game_help;
mod game_logic;
mod game_room;
mod game_state;
mod id;
mod line;
mod message_stash;
mod named;
mod server_actor;
mod server_websocket;
mod text_util;

use std::{fs, net::SocketAddr};

use game_state::{IdMap, LoadedGameState, MobTemplate, Room};
use id::Id;
use serde::de::DeserializeOwned;
use server_websocket::{handle_connection, ConnectQuery};
use tokio::sync::mpsc;
use warp::Filter;

#[tokio::main]
async fn main() {
    env_logger::init();

    let loaded_game_state = LoadedGameState {
        rooms: load_id_map("data/rooms.yaml", |r: &Room| r.id),
        mob_templates: load_id_map("data/mob_templates.yaml", |mt: &MobTemplate| mt.id),
    };

    let socket_address = std::env::var("MUD_ADDR")
        .ok()
        .and_then(|str| str.parse::<SocketAddr>().ok())
        .unwrap_or_else(|| ([127, 0, 0, 1], 8081).into());

    let (actor_sender, actor_receiver) = mpsc::channel::<server_actor::Message>(4096);
    {
        let actor_sender = actor_sender.clone();
        tokio::spawn(async move {
            server_actor::run(actor_receiver, actor_sender, loaded_game_state).await
        });
    }

    let routes = warp::path!("api" / "ws")
        .and(warp::query::<ConnectQuery>())
        .and(warp::ws())
        .map(move |query: ConnectQuery, ws: warp::ws::Ws| {
            let message_sender = actor_sender.clone();
            ws.on_upgrade(|websocket| handle_connection(websocket, query, message_sender))
        });

    warp::serve(routes).run(socket_address).await;
}

fn load_id_map<T>(path: &str, to_id: impl Fn(&T) -> Id<T>) -> IdMap<T>
where
    T: DeserializeOwned,
{
    let list: Vec<T> = serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    list.into_iter().map(|item| (to_id(&item), item)).collect()
}
