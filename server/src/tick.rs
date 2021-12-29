use std::{ops::Add, time::Duration};

use rand::Rng;
use serde::{Deserialize, Deserializer};

pub static TICK_INTERVAL: Duration = Duration::from_millis(1000 / LARGE_TICK_FREQUENCY as u64);
static LARGE_TICK_FREQUENCY: i64 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick(i64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TickDuration(i64);

impl Tick {
    pub fn zero() -> Tick {
        Tick(0)
    }

    pub fn is_large_tick(&self) -> bool {
        self.0 % LARGE_TICK_FREQUENCY == 0
    }

    pub fn is_on_division(&self, divide: TickDuration, offset: TickDuration) -> bool {
        self.0 % divide.0 == offset.0
    }

    pub fn increase(&self) -> Tick {
        Tick(self.0 + 1)
    }
}

impl TickDuration {
    pub fn from_secs(secs: f32) -> TickDuration {
        TickDuration((secs / TICK_INTERVAL.as_secs_f32()) as i64)
    }

    pub fn deserialize_from_secs<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        f32::deserialize(deserializer).map(TickDuration::from_secs)
    }

    pub fn random_offset(&self, rng: &mut impl Rng) -> TickDuration {
        TickDuration(rng.gen_range(0..self.0))
    }
}

impl Add<TickDuration> for Tick {
    type Output = Tick;

    fn add(self, rhs: TickDuration) -> Self::Output {
        Tick(self.0 + rhs.0)
    }
}
