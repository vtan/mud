use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Deserialize;

use crate::{
    id::{Id, IdSource},
    named::Named,
};

pub type IdMap<T> = HashMap<Id<T>, T>;

pub struct LoadedGameState {
    pub rooms: IdMap<Room>,
    pub mob_templates: IdMap<MobTemplate>,
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub ticks: u64,
    pub players: IdMap<Player>,
    pub rooms: IdMap<Room>,
    pub room_vars: HashMap<(Id<Room>, String), i32>,
    pub scheduled_room_var_resets: BTreeMap<u64, (Id<Room>, String, String)>,
    pub mob_templates: IdMap<MobTemplate>,
    pub mob_instances: IdMap<MobInstance>,
    pub mob_instance_id_source: IdSource<MobInstance>,
    pub scheduled_mob_spawns: BTreeMap<u64, (Id<Room>, Id<MobTemplate>)>,
}

impl GameState {
    pub fn new(loaded_game_state: LoadedGameState) -> GameState {
        let LoadedGameState { rooms, mob_templates } = loaded_game_state;
        GameState {
            rooms,
            mob_templates,
            ticks: 0,
            players: HashMap::new(),
            room_vars: HashMap::new(),
            scheduled_room_var_resets: BTreeMap::new(),
            mob_instances: HashMap::new(),
            mob_instance_id_source: IdSource::new(0),
            scheduled_mob_spawns: BTreeMap::new(),
        }
    }

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
    pub hp: i32,
    pub attack_target: Option<Id<MobInstance>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub id: Id<Room>,
    pub name: String,
    pub description: RoomDescription,
    pub exits: HashMap<String, RoomExit>,
    #[serde(default)]
    pub objects: Vec<RoomObject>,
    #[serde(default)]
    pub mob_spawns: Vec<MobSpawn>,
}

impl Room {
    pub fn exit_direction_to(&self, room_id: Id<Room>) -> Option<&str> {
        self.exits.iter().find_map(|(direction, exit)| match exit {
            RoomExit::Static(to) if room_id == *to => Some(direction.as_str()),
            RoomExit::Conditional { to, .. } if room_id == *to => Some(direction.as_str()),
            _ => None,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum RoomExit {
    Static(Id<Room>),
    Conditional { condition: Condition, to: Id<Room> },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomObject {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: RoomDescription,
    #[serde(default)]
    pub commands: Vec<RoomCommand>,
}

impl Named for RoomObject {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_aliases(&self) -> &[String] {
        &self.aliases
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RoomDescription {
    Static(String),
    Dynamic(Vec<DynamicDescriptionFragment>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicDescriptionFragment {
    pub fragment: String,
    pub condition: Option<Condition>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobSpawn {
    pub mob_template_id: Id<MobTemplate>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobTemplate {
    pub id: Id<MobTemplate>,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: String,
    pub max_hp: i32,
    pub damage: i32,
}

impl Named for MobTemplate {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_aliases(&self) -> &[String] {
        &self.aliases
    }
}

#[derive(Clone, Debug)]
pub struct MobInstance {
    pub id: Id<MobInstance>,
    pub room_id: Id<Room>,
    pub template: MobTemplate,
    pub hp: i32,
    pub hostile_to: HashSet<Id<Player>>,
    pub attack_target: Option<Id<Player>>,
}
