use std::collections::HashMap;

use crate::{id::Id, line::Line, player::Player};

pub struct EventWriter {
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

    pub fn tell_lines(&mut self, player_id: Id<Player>, lines: &[Line]) {
        if let Some(existing) = self.lines.get_mut(&player_id) {
            existing.extend_from_slice(lines);
        } else {
            self.lines.insert(player_id, lines.to_vec());
        }
    }

    pub fn tell_many(&mut self, player_ids: impl Iterator<Item = Id<Player>>, line: Line) {
        // TODO: store a Vec<Rc/Arc<Line>> to avoid storing the same Line many times?
        for player_id in player_ids {
            self.tell(player_id, line.clone());
        }
    }
}
