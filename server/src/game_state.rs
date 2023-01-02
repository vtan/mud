use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

use crate::{
    id::{Id, IdMap, IdSource},
    mob::{MobInstance, MobSpawn, MobTemplate},
    mob_coll::MobColl,
    named::Named,
    tick::{Tick, TickDuration},
};

pub struct LoadedGameState {
    pub rooms: IdMap<Room>,
    pub mob_templates: IdMap<MobTemplate>,
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub ticks: Tick,
    pub players: IdMap<Player>,
    pub rooms: IdMap<Room>,
    pub room_vars: HashMap<(Id<Room>, String), i32>,
    pub scheduled_room_var_resets: BTreeMap<Tick, (Id<Room>, String, String)>,
    pub mob_templates: IdMap<MobTemplate>,
    pub mob_instances: MobColl,
    pub mob_instance_id_source: IdSource<MobInstance>,
    pub scheduled_mob_spawns: BTreeMap<Tick, (Id<Room>, Id<MobTemplate>)>,
}

impl GameState {
    pub fn new(loaded_game_state: LoadedGameState) -> GameState {
        let LoadedGameState { rooms, mob_templates } = loaded_game_state;
        GameState {
            rooms,
            mob_templates,
            ticks: Tick::zero(),
            players: HashMap::new(),
            room_vars: HashMap::new(),
            scheduled_room_var_resets: BTreeMap::new(),
            mob_instances: MobColl::new(),
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

pub fn player_ids_in_room(
    players: &IdMap<Player>,
    room_id: Id<Room>,
) -> impl Iterator<Item = Id<Player>> + '_ {
    players.values().filter_map(move |player| {
        if player.room_id == room_id {
            Some(player.id)
        } else {
            None
        }
    })
}

pub fn player_ids_in_room_except(
    players: &IdMap<Player>,
    room_id: Id<Room>,
    except: Id<Player>,
) -> impl Iterator<Item = Id<Player>> + '_ {
    players.values().filter_map(move |player| {
        if player.room_id == room_id && player.id != except {
            Some(player.id)
        } else {
            None
        }
    })
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: Id<Player>,
    pub name: String,
    pub room_id: Id<Room>,
    pub hp: i32,
    pub max_hp: i32,
    pub attack_offset: TickDuration,
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
    ResetRoomVarAfterSecs(String, f32, String),
    TellSelf(String),
    TellOthers(String),
    TellRoom(String),
}
