use crate::{game_state::Room, id::Id, mob::Mob, tick::TickDuration};

#[derive(Clone, Debug)]
pub struct Player {
    pub id: Id<Player>,
    pub name: String,
    pub room_id: Id<Room>,
    pub hp: i32,
    pub max_hp: i32,
    pub attack_offset: TickDuration,
    pub attack_target: Option<Id<Mob>>,
}
