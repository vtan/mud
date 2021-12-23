use std::collections::HashMap;

use crate::{
    game_state::{GameState, Player, Room},
    id::Id,
    line::{Line, LineSpan},
};

pub fn on_command(
    player_id: Id<Player>,
    words: Vec<&str>,
    game_state: &mut GameState,
) -> HashMap<Id<Player>, Vec<Line>> {
    let mut events = HashMap::new();

    if let Some(player) = game_state.players.get(&player_id) {
        if let &["look"] = &words[..] {
            if let Some(room) = game_state.rooms.get(&player.room_id) {
                events = describe_room(player_id, room);
            }
        } else {
            if let Some(exit_room_id) = words.get(0).and_then(|exit| {
                game_state
                    .rooms
                    .get(&player.room_id)
                    .and_then(|room| room.exits.get(&exit.to_string()).copied())
            }) {
                if let Some(exit_room) = game_state.rooms.get(&exit_room_id) {
                    if let Some(player) = game_state.players.get_mut(&player_id) {
                    player.room_id = exit_room_id;
                    events = describe_room(player_id, exit_room);
                    }
                }
            } else {
                events.insert(player_id, vec!["Unknown command.".into()]);
            }
        }
    }

    events
}

fn describe_room(player_id: Id<Player>, room: &Room) -> HashMap<Id<Player>, Vec<Line>> {
    let mut result = HashMap::new();
    let exits = if room.exits.is_empty() {
        "There are no exits here.".to_string()
    } else {
        format!("You can go {} from here.", and_list(&room.exits.keys().cloned().collect::<Vec<_>>()))
    };
    result.insert(
        player_id,
        vec![
            Line {
                spans: vec![LineSpan {
                    text: room.name.clone(),
                    bold: Some(true),
                }],
            },
            room.description.clone().into(),
            exits.into(),
        ],
    );
    result
}

fn and_list(words: &[String]) -> String {
    match words.len() {
        0 => "".to_string(),
        1 => words[0].clone(),
        2 => format!("{} and {}", words[0], words[1]),
        len => format!("{} and {}", words[0..len-1].join(", "), words[len-1])
    }
}
