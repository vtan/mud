use std::collections::HashMap;

use serde::Deserialize;

use crate::id::Id;

#[derive(Clone, Debug)]
pub struct GameState {
    pub players: HashMap<Id<Player>, Player>,
    pub rooms: HashMap<Id<Room>, Room>,
    pub room_vars: HashMap<(Id<Room>, String), i32>,
}

impl GameState {
    pub fn get_room_var(&self, room_id: Id<Room>, var: String) -> i32 {
        *self.room_vars.get(&(room_id, var)).unwrap_or(&0)
    }

    pub fn set_room_var(&mut self, room_id: Id<Room>, var: String, value: i32) {
        if value == 0 {
            self.room_vars.remove(&(room_id, var));
        } else {
            self.room_vars.insert((room_id, var), value);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: Id<Player>,
    pub name: String,
    pub room_id: Id<Room>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Room {
    pub id: Id<Room>,
    pub name: String,
    pub description: String,
    pub exits: HashMap<String, Id<Room>>,
    #[serde(default)]
    pub objects: Vec<RoomObject>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RoomObject {
    pub name: String,
    pub description: RoomObjectDescription,
    #[serde(default)]
    pub commands: Vec<RoomCommand>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RoomObjectDescription {
    Constant(String),
    Conditional(Vec<ConditionalDescription>),
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConditionalDescription {
    pub condition: Condition,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RoomCommand {
    pub command: String,
    #[serde(default)]
    pub condition: Option<Condition>,
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Condition {
    Equals(String, i32),
    NotEquals(String, i32),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Statement {
    SetRoomVar(String, i32),
    TellSelf(String),
    TellOthers(String),
    TellRoom(String),
}
