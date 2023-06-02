use std::sync::{Arc};
use tokio::time::Instant;
use parking_lot::Mutex;

#[derive(Clone)]
pub enum Clock {
    Real,
    Fake(Arc<FakeClock>),
}

pub struct FakeClock(Mutex<Instant>);

impl FakeClock {
    pub fn set_now(&self, time: Instant) { *self.0.lock() = time }
    pub fn now(&self) -> Instant { *self.0.lock() }
}

impl Clock {
    pub fn now(&self) -> Instant {
        match self {
            Clock::Real => Instant::now(),
            Clock::Fake(x) => x.now(),
        }
    }
}