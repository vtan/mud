use std::collections::HashMap;

use futures_util::future;
use log::debug;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::{
    event_writer::EventWriter,
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

pub async fn run(mut messages: mpsc::Receiver<Message>, rooms: HashMap<Id<Room>, Room>) {
    let mut connections: HashMap<Id<Player>, _> = HashMap::new();
    let mut game_state = GameState {
        players: HashMap::new(),
        rooms,
    };
    let mut event_writer = EventWriter {
        lines: HashMap::new(),
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
                connections.insert(player_id, connection);
                let player = Player {
                    id: player_id,
                    name: player_name,
                    room_id: Id::new(0),
                };
                game_logic::on_player_connect(player, &mut event_writer, &mut game_state);
            }
            PlayerDisconnected { player_id } => {
                connections.remove(&player_id);
                game_logic::on_player_disconnect(player_id, &mut event_writer, &mut game_state);
            }
            PlayerCommand { player_id, command } => {
                game_logic::on_command(player_id, &command, &mut event_writer, &mut game_state);
            }
        }
        send_player_events(&connections, &mut event_writer).await;
    }
}

async fn send_player_events(
    connections: &HashMap<Id<Player>, mpsc::Sender<PlayerEvent>>,
    event_writer: &mut EventWriter
) {
    future::try_join_all(event_writer.lines.iter().filter_map(|(player_id, lines)| {
        if let Some(connection) = connections.get(player_id) {
            let event = PlayerEvent {
                lines: lines.clone(),
            };
            Some(connection.send(event))
        } else {
            None
        }
    }))
    .await
    .unwrap();
    event_writer.lines.clear();
}
