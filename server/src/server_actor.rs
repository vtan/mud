use std::collections::{HashMap, HashSet};

use futures_util::future;
use log::{debug, warn};
use rand::thread_rng;
use serde::Serialize;
use tokio::{sync::mpsc, time};

use crate::{
    event_writer::EventWriter,
    game_combat, game_logic,
    game_state::{GameState, LoadedGameState, Player, Room},
    id::Id,
    line::Line,
    tick,
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
    Tick,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerEvent {
    lines: Vec<Line>,
    room_info: Option<RoomInfo>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoomInfo {
    self_player: EntityInfo,
    players: Vec<EntityInfo>,
    mobs: Vec<EntityInfo>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntityInfo {
    id: String,
    name: String,
    hp: i32,
    max_hp: i32,
}

pub async fn run(
    mut messages: mpsc::Receiver<Message>,
    self_sender: mpsc::Sender<Message>,
    loaded_game_state: LoadedGameState,
) {
    use Message::*;

    tokio::spawn(async move {
        let mut interval = time::interval(tick::TICK_INTERVAL);
        loop {
            interval.tick().await;
            self_sender.send(Tick).await.unwrap();
        }
    });

    let mut connections: HashMap<Id<Player>, _> = HashMap::new();
    let mut game_state = GameState::new(loaded_game_state);
    let mut event_writer =
        EventWriter { lines: HashMap::new(), room_entities_changed: HashSet::new() };

    game_logic::initialize(&mut game_state);

    debug!("Server loop starting");
    while let Some(message) = messages.recv().await {
        match message {
            PlayerConnected { player_id, player_name, connection } => {
                connections.insert(player_id, connection);
                let player = Player {
                    id: player_id,
                    name: player_name,
                    room_id: Id::new(0),
                    hp: 100,
                    max_hp: 100,
                    attack_offset: game_combat::PLAYER_ATTACK_FREQ.random_offset(&mut thread_rng()),
                    attack_target: None,
                };
                game_logic::on_player_connect(player, &mut event_writer, &mut game_state);
            }
            PlayerDisconnected { player_id } => {
                connections.remove(&player_id);
                game_logic::on_player_disconnect(player_id, &mut event_writer, &mut game_state);
            }
            PlayerCommand { player_id, command } => {
                if let Err(err) =
                    game_logic::on_command(player_id, &command, &mut event_writer, &mut game_state)
                {
                    warn!("Player command: {}", err);
                }
            }
            Tick => {
                game_logic::on_tick(&mut event_writer, &mut game_state);
            }
        }
        send_player_events(&game_state, &connections, &mut event_writer).await;
    }
}

async fn send_player_events(
    state: &GameState,
    connections: &HashMap<Id<Player>, mpsc::Sender<PlayerEvent>>,
    event_writer: &mut EventWriter,
) {
    let room_infos = event_writer
        .room_entities_changed
        .iter()
        .flat_map(|room_id| collect_room_info(*room_id, state))
        .collect::<HashMap<_, _>>();

    let player_ids = room_infos.keys().chain(event_writer.lines.keys()).collect::<HashSet<_>>();

    future::try_join_all(player_ids.into_iter().filter_map(|player_id| {
        if let Some(connection) = connections.get(player_id) {
            let lines = event_writer.lines.get(player_id).cloned().unwrap_or_default();
            let room_info = room_infos.get(player_id).cloned();
            let event = PlayerEvent { lines, room_info };
            Some(connection.send(event))
        } else {
            None
        }
    }))
    .await
    .unwrap();

    event_writer.lines.clear();
    event_writer.room_entities_changed.clear();
}

fn collect_room_info(room_id: Id<Room>, state: &GameState) -> Vec<(Id<Player>, RoomInfo)> {
    let (player_ids, players): (Vec<_>, Vec<_>) = state
        .players
        .values()
        .filter_map(|p| {
            if p.room_id == room_id {
                Some((
                    p.id,
                    EntityInfo {
                        id: p.id.value.to_string(),
                        name: p.name.clone(),
                        hp: p.hp,
                        max_hp: p.max_hp,
                    },
                ))
            } else {
                None
            }
        })
        .unzip();
    let mobs = state
        .mob_instances
        .by_id()
        .values()
        .filter_map(|m| {
            if m.room_id == room_id {
                Some(EntityInfo {
                    id: m.id.value.to_string(),
                    name: m.template.name.clone(),
                    hp: m.hp,
                    max_hp: m.template.max_hp,
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    players
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, self_player)| {
            let mut players = players.clone();
            players.remove(i);
            let mobs = mobs.clone();
            let room_info = RoomInfo { self_player, players, mobs };
            (player_ids[i], room_info)
        })
        .collect()
}
