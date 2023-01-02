use std::collections::HashSet;

use serde::Deserialize;

use crate::{
    game_state::{Player, Room},
    id::Id,
    named::Named,
    tick::TickDuration,
};

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
    #[serde(deserialize_with = "TickDuration::deserialize_from_secs")]
    pub attack_period: TickDuration,
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
    pub attack_offset: TickDuration,
    pub hostile_to: HashSet<Id<Player>>,
    pub attack_target: Option<Id<Player>>,
}
