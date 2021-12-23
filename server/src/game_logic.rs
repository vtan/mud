use std::collections::HashMap;

use crate::{
    game_state::{GameState, Player},
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
                events.insert(
                    player.id,
                    vec![
                        Line {
                            spans: vec![LineSpan {
                                text: room.name.clone(),
                                bold: Some(true),
                            }],
                        },
                        room.description.clone().into(),
                    ],
                );
            }
        } else {
            events.insert(player_id, vec!("Unknown command.".into()));
        }
    }

    events
}
