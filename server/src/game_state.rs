use std::collections::HashMap;

use serde::Deserialize;

use crate::id::Id;

#[derive(Clone, Debug)]
pub struct GameState {
    pub players: HashMap<Id<Player>, Player>,
    pub rooms: HashMap<Id<Room>, Room>
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
    pub description: String,
}
