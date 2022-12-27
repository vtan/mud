use std::collections::HashMap;

use crate::{
    game_state::{GameState, Player, Room},
    id::Id,
    line::Line,
};

pub struct EventWriter {
    // TODO: store a Vec<Rc/Arc<Line>> to avoid storing the same Line many times?
    pub lines: HashMap<Id<Player>, Vec<Line>>,
}

impl EventWriter {
    pub fn tell(&mut self, player_id: Id<Player>, line: Line) {
        if let Some(existing) = self.lines.get_mut(&player_id) {
            existing.push(line);
        } else {
            self.lines.insert(player_id, vec![line]);
        }
    }

    pub fn tell_many(&mut self, player_id: Id<Player>, lines: &[Line]) {
        if let Some(existing) = self.lines.get_mut(&player_id) {
            existing.extend_from_slice(lines);
        } else {
            self.lines.insert(player_id, lines.to_vec());
        }
    }

    pub fn tell_multi(&mut self, player_ids: impl Iterator<Item = Id<Player>>, line: Line) {
        for player_id in player_ids {
            self.tell(player_id, line.clone());
        }
    }

    pub fn tell_room(&mut self, line: Line, room_id: Id<Room>, state: &GameState) {
        state.players.values().for_each(|player| {
            if player.room_id == room_id {
                self.tell(player.id, line.clone());
            }
        })
    }

    pub fn tell_room_except(
        &mut self,
        line: Line,
        room_id: Id<Room>,
        except: Id<Player>,
        state: &GameState,
    ) {
        state.players.values().for_each(|player| {
            if player.id != except && player.room_id == room_id {
                self.tell(player.id, line.clone());
            }
        })
    }
}
