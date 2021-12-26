use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

use crate::id::Id;

#[derive(Clone, Debug)]
pub struct GameState {
    pub ticks: u64,
    pub players: HashMap<Id<Player>, Player>,
    pub rooms: HashMap<Id<Room>, Room>,
    pub room_vars: HashMap<(Id<Room>, String), i32>,
    pub scheduled_room_var_resets: BTreeMap<u64, (Id<Room>, String, String)>,
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
    pub description: RoomDescription,
    pub exits: HashMap<String, RoomExit>,
    #[serde(default)]
    pub objects: Vec<RoomObject>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RoomExit {
    Static(Id<Room>),
    Conditional { condition: Condition, to: Id<Room> },
}

#[derive(Clone, Debug, Deserialize)]
pub struct RoomObject {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: RoomDescription,
    #[serde(default)]
    pub commands: Vec<RoomCommand>,
}

impl RoomObject {
    pub fn matches(&self, str: &str) -> bool {
        self.name.eq_ignore_ascii_case(str)
            || self.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(str))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RoomDescription {
    Static(String),
    Dynamic(Vec<DynamicDescriptionFragment>),
}

#[derive(Clone, Debug, Deserialize)]
pub struct DynamicDescriptionFragment {
    pub fragment: String,
    pub condition: Option<Condition>,
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
    ResetRoomVarAfterTicks(String, u64, String),
    TellSelf(String),
    TellOthers(String),
    TellRoom(String),
}
