use std::cmp::max;
use std::collections::VecDeque;
use std::future::Future;
use std::time::{Duration};
use futures::future::BoxFuture;
use parking_lot::Mutex;
use tokio::time::Instant;
use crate::stream::FuturesUnordered;
use crate::util::average::RunningAverage;
use crate::util::clock::Clock;

pub struct RateLimit {
    history: VecDeque<Instant>,
    window: usize,
    clock: Clock,
    target_rate: f64,
    start: Instant,
}

impl RateLimit {
    pub fn new(clock: Clock, window: usize, target_rate: f64) -> Self {
        RateLimit {
            history: VecDeque::new(),
            window,
            target_rate,
            start: clock.now(),
            clock,
        }
    }
    pub fn spawn(&mut self) -> Instant {
        let mut time = self.clock.now();
        if let Some(front) = self.history.front() {
            if let Some(new_time) = front.checked_add(Duration::from_secs_f64(self.history.len() as f64 / self.target_rate)) {
                time = max(time, new_time);
            }
        }
        self.history.push_back(time);
        if self.history.len() > self.window {
            self.history.pop_front();
        }
        time
    }
}

#[tokio::test]
async fn test_rate_limit() {
    let mut rate = RateLimit::new(Clock::Real, 10, 10.0);
    for i in 0..1000 {
        println!("{:?} {:?}", i, Instant::now());
        tokio::time::sleep_until(rate.spawn()).await;
    }
}

//
// pub struct RateLimit {
//     successes: RunningAverage,
//     failures: RunningAverage,
//     running: usize,
//     clock: Clock,
// }
//
// pub enum RateLimitError {
//     Throttled,
//     Generic,
// }
//
// impl RateLimit {
//     pub fn new(clock: Clock, param: f64) -> Self {
//         let (queue_tx, mut queue_rx) =
//             tokio::sync::mpsc::unbounded_channel::<BoxFuture<RateLimitError>>();
//         tokio::spawn(async move {
//             let successes = RunningAverage::new(param);
//             let failures = RunningAverage::new(param);
//             let mut tasks = FuturesUnordered::new();
//             while let Some(task) = queue_rx.recv().await {
//                 tasks.push(task);
//             }
//         });
//         RateLimit {
//             successes: RunningAverage::new(param),
//             failures: RunningAverage::new(param),
//             running: 0,
//             clock,
//         }
//     }
//     pub fn spawn<F: Future>(&self, fut: F) -> F::Output {}
//     async fn run_with_retries<'a, T>(&self, f: &dyn Fn() -> BoxFuture<'a, Result<T, RateLimitError>>) -> T {
//         tokio::time::sleep(self.0.lock().start()).await;
//         loop {
//             let result = f().await;
//             let mut lock = self.0.lock();
//             match result {
//                 Ok(x) => return x,
//                 Err(e) => lock.end(e),
//             }
//             tokio::time::sleep(lock.start()).await
//         }
//     }
// }