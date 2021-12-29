use std::collections::HashMap;

use crate::{
    event_writer::EventWriter,
    game_state::{IdMap, Player, Room},
    id::Id,
    line::Line,
};

pub struct MessageStash {
    messages: Vec<Message>,
}

enum Message {
    ToPlayer { player_id: Id<Player>, line: Line },
    ToRoom { room_id: Id<Room>, except: Option<Id<Player>>, line: Line },
}

impl MessageStash {
    pub fn new() -> MessageStash {
        MessageStash { messages: Vec::new() }
    }

    pub fn tell(&mut self, player_id: Id<Player>, line: Line) {
        self.messages.push(Message::ToPlayer { player_id, line });
    }

    pub fn tell_room(&mut self, room_id: Id<Room>, line: Line) {
        self.messages.push(Message::ToRoom { room_id, line, except: None });
    }

    pub fn tell_room_except(&mut self, room_id: Id<Room>, except: Id<Player>, line: Line) {
        self.messages.push(Message::ToRoom { room_id, line, except: Some(except) });
    }

    pub fn write_into(self, event_writer: &mut EventWriter, players: &IdMap<Player>) {
        let mut players_in_rooms = HashMap::new();
        self.messages.into_iter().for_each(|message| match message {
            Message::ToPlayer { player_id, line } => event_writer.tell(player_id, line),
            Message::ToRoom { room_id, except, line } => {
                let players_in_room = players_in_rooms.entry(room_id).or_insert_with(|| {
                    players
                        .values()
                        .filter(|player| player.room_id == room_id)
                        .map(|player| player.id)
                        .collect::<Vec<_>>()
                });
                players_in_room.iter().for_each(|player_id| {
                    if except != Some(*player_id) {
                        event_writer.tell(*player_id, line.clone());
                    }
                });
            }
        });
    }
}
