use std::collections::HashMap;

use futures_util::future;
use log::debug;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::{
    game_logic,
    game_state::{GameState, Player, Room},
    id::Id,
    line::Line,
};

#[derive(Debug)]
pub enum Message {
    PlayerConnected {
        player_id: Id<Player>,
        player_name: String,
        connection: mpsc::Sender<PlayerEvent>,
    },
    PlayerDisconnected {
        player_id: Id<Player>,
    },
    PlayerCommand {
        player_id: Id<Player>,
        command: String,
    },
}

#[derive(Serialize, Debug)]
pub struct PlayerEvent {
    lines: Vec<Line>,
}

struct PlayerState {
    connection: mpsc::Sender<PlayerEvent>,
}

pub async fn run(mut messages: mpsc::Receiver<Message>, rooms: HashMap<Id<Room>, Room>) {
    let mut players: HashMap<Id<Player>, _> = HashMap::new();
    let mut game_state = GameState {
        players: HashMap::new(),
        rooms,
    };

    debug!("Server loop starting");
    use Message::*;
    while let Some(message) = messages.recv().await {
        match message {
            PlayerConnected {
                player_id,
                player_name,
                connection,
            } => {
                connection
                    .send(PlayerEvent {
                        lines: vec![format!("Welcome, {}!", player_name).into()],
                    })
                    .await
                    .unwrap();
                let player_state = PlayerState { connection };
                players.insert(player_id, player_state);
                let player = Player {
                    id: player_id,
                    name: player_name,
                    room_id: Id::new(0),
                };
                game_state.players.insert(player_id, player);
            }
            PlayerDisconnected { player_id } => {
                players.remove(&player_id);
            }
            PlayerCommand { player_id, command } => {
                let words: Vec<&str> = command.split_whitespace().collect();
                let events = game_logic::on_command(player_id, words, &mut game_state);
                future::try_join_all(
                    events.iter().filter_map(|(player_id, lines)| {
                        if let Some(connection) = players.get(player_id) {
                            let event = PlayerEvent {
                                lines: lines.clone(),
                            };
                            Some(connection.connection.send(event))
                        } else {
                            None
                        }
                    })
                ).await.unwrap();
            }
        }
    }
}
