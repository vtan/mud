use std::collections::{hash_map::Entry, HashMap};

use crate::{
    game_state::Room,
    id::{Id, IdMap},
    mob::Mob,
};

#[derive(Debug, Clone)]
pub struct MobColl {
    by_id: IdMap<Mob>,
    by_room_id: HashMap<Id<Room>, Vec<Id<Mob>>>,
}

impl MobColl {
    pub fn new() -> Self {
        Self { by_id: HashMap::new(), by_room_id: HashMap::new() }
    }

    pub fn by_id(&self) -> &IdMap<Mob> {
        &self.by_id
    }

    pub fn by_room_id(&self) -> &HashMap<Id<Room>, Vec<Id<Mob>>> {
        &self.by_room_id
    }

    pub fn insert(&mut self, mob: Mob) {
        let Mob { id, room_id, .. } = mob;
        if self.by_id.insert(id, mob).is_some() {
            unreachable!();
        }
        self.add_to_room_index(id, room_id);
    }

    pub fn modify<T>(&mut self, id: &Id<Mob>, f: impl FnOnce(&mut Mob) -> T) -> T {
        if let Some(mob) = self.by_id.get_mut(id) {
            let before = mob.clone();
            let result = f(mob);
            let Mob { room_id: after_room_id, .. } = *mob;

            if before.room_id != after_room_id {
                self.remove_from_room_index(*id, before.room_id);
                self.add_to_room_index(*id, after_room_id);
            }

            result
        } else {
            unreachable!();
        }
    }

    pub fn remove(&mut self, id: &Id<Mob>) -> Option<Mob> {
        if let Some(removed) = self.by_id.remove(id) {
            self.remove_from_room_index(*id, removed.room_id);
            Some(removed)
        } else {
            None
        }
    }

    fn add_to_room_index(&mut self, mob_id: Id<Mob>, room_id: Id<Room>) {
        self.by_room_id.entry(room_id).or_default().push(mob_id);
    }

    fn remove_from_room_index(&mut self, mob_id: Id<Mob>, room_id: Id<Room>) {
        let entry = self
            .by_room_id
            .entry(room_id)
            .and_modify(|ids| ids.retain(|id_in_room| *id_in_room != mob_id));
        match entry {
            Entry::Occupied(e) => {
                if e.get().is_empty() {
                    e.remove_entry();
                }
            }
            _ => unreachable!(),
        }
    }
}
