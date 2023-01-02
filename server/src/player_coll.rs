use std::collections::{hash_map::Entry, HashMap};

use crate::{
    game_state::Room,
    id::{Id, IdMap},
    player::Player,
};

#[derive(Debug, Clone)]
pub struct PlayerColl {
    by_id: IdMap<Player>,
    by_room_id: HashMap<Id<Room>, Vec<Id<Player>>>,
}

impl PlayerColl {
    pub fn new() -> Self {
        Self { by_id: HashMap::new(), by_room_id: HashMap::new() }
    }

    pub fn by_id(&self) -> &IdMap<Player> {
        &self.by_id
    }

    pub fn by_room_id(&self) -> &HashMap<Id<Room>, Vec<Id<Player>>> {
        &self.by_room_id
    }

    pub fn ids_in_room(&self, room_id: Id<Room>) -> impl Iterator<Item = Id<Player>> + '_ {
        self.by_room_id.get(&room_id).into_iter().flat_map(|ids| ids.iter()).copied()
    }

    pub fn ids_in_room_except(
        &self,
        room_id: Id<Room>,
        except: Id<Player>,
    ) -> impl Iterator<Item = Id<Player>> + '_ {
        self.by_room_id
            .get(&room_id)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter(move |id| **id != except)
            .copied()
    }

    pub fn insert(&mut self, player: Player) {
        let Player { id, room_id, .. } = player;
        if self.by_id.insert(id, player).is_some() {
            unreachable!();
        }
        self.add_to_room_index(id, room_id);
    }

    pub fn modify<T>(&mut self, id: &Id<Player>, f: impl FnOnce(&mut Player) -> T) -> T {
        if let Some(mob) = self.by_id.get_mut(id) {
            let before = mob.clone();
            let result = f(mob);
            let Player { room_id: after_room_id, .. } = *mob;

            if before.room_id != after_room_id {
                self.remove_from_room_index(*id, before.room_id);
                self.add_to_room_index(*id, after_room_id);
            }

            result
        } else {
            unreachable!();
        }
    }

    pub fn remove(&mut self, id: &Id<Player>) -> Option<Player> {
        if let Some(removed) = self.by_id.remove(id) {
            self.remove_from_room_index(*id, removed.room_id);
            Some(removed)
        } else {
            None
        }
    }

    fn add_to_room_index(&mut self, player_id: Id<Player>, room_id: Id<Room>) {
        self.by_room_id.entry(room_id).or_default().push(player_id);
    }

    fn remove_from_room_index(&mut self, player_id: Id<Player>, room_id: Id<Room>) {
        let entry = self
            .by_room_id
            .entry(room_id)
            .and_modify(|ids| ids.retain(|id_in_room| *id_in_room != player_id));
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
