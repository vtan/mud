use std::collections::HashMap;

use crate::{
    id::{Id, IdMap},
    mob::Mob,
};

#[derive(Debug, Clone)]
pub struct MobColl {
    by_id: IdMap<Mob>,
}

impl MobColl {
    pub fn new() -> Self {
        Self { by_id: HashMap::new() }
    }

    pub fn by_id(&self) -> &IdMap<Mob> {
        &self.by_id
    }

    pub fn insert(&mut self, mob: Mob) {
        if self.by_id.insert(mob.id, mob).is_some() {
            unreachable!();
        }
    }

    pub fn modify<T>(&mut self, id: &Id<Mob>, f: impl FnOnce(&mut Mob) -> T) -> T {
        if let Some(mob) = self.by_id.get_mut(id) {
            f(mob)
        } else {
            unreachable!();
        }
    }

    pub fn remove(&mut self, id: &Id<Mob>) {
        self.by_id.remove(id);
    }
}
