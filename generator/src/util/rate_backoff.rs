use std::cmp::max;
use std::collections::VecDeque;
use std::sync::{Arc};
use std::time::{Duration};
use parking_lot::Mutex;
use rand::{Rng, thread_rng};
use tokio::time::Instant;
use crate::util::clock::Clock;
use safe_once::cell::OnceCell;
use tokio::sync::Semaphore;
use safe_once::sync::OnceLock;

pub struct Event {
    pub time: Instant,
    pub success: OnceLock<bool>,
}

pub struct RateBackoff {
    history: VecDeque<Arc<Event>>,
    window: usize,
    clock: Clock,
    error_budget: f64,
    max_rate: f64,
}

impl RateBackoff {
    pub fn new(window: usize, error_budget: f64, max_rate: f64) -> Self {
        RateBackoff {
            history: VecDeque::new(),
            window,
            clock: Clock::Real,
            error_budget,
            max_rate,
        }
    }
    pub fn spawn(&mut self) -> Arc<Event> {
        let time = self.clock.now();
        let mut delay = Duration::ZERO;
        if self.history.len() > self.window / 10 {
            let successes = self.history.iter().filter(|x| x.success.get().map_or(false, |x| *x)).count();
            let failures = self.history.iter().filter(|x| x.success.get().map_or(false, |x| !*x)).count();
            let total = successes + failures;
            if successes > 0 {
                let failure_rate = (failures as f64) / (total as f64);
                let front = self.history.front().unwrap();
                let past = time - front.time;
                let past_average = past / (self.history.len() as u32 - 1);
                let time_between_successes = past / (successes as u32);
                delay = Duration::from_secs_f64(time_between_successes.as_secs_f64() / (1.0 + self.error_budget));
                // dbg!(delay, successes, past,time_between_successes);
                println!("successes={successes:?} failures={failures:?} delay={delay:?} past={:?}", past);
                // let delay_factor = (failure_rate - self.error_budget) ;
                // let delay =
                //     Duration::from_secs_f64(past_average)*(1.0+delay_factor*0.1)
                //         +delay_factor*;
                // time = time + delay;
                // println!("{:?} {:?}", delay, delay_factor);
            } else {
                delay = Duration::from_secs_f64((failures as f64).powf(1.1));
                println!("no successes: {:?}", delay);
            }
        }
        if let Some(front) = self.history.front() {
            delay = max(delay, Duration::from_secs_f64(self.history.len() as f64 / self.max_rate));
        }
        let time = time + delay;
        let event = Arc::new(Event { time, success: OnceLock::new() });
        self.history.push_back(event.clone());
        if self.history.len() > self.window {
            self.history.pop_front();
        }
        event
    }
}

#[tokio::test]
async fn test_rate_backoff() {
    let mut rate = Arc::new(Mutex::new(RateBackoff::new(100, 0.5, 10.)));
    let semaphore = Arc::new(Semaphore::new(5));
    let mut handles = vec![];
    for i in 0..1000 {
        let semaphore = semaphore.clone();
        let rate = rate.clone();
        handles.push(tokio::spawn(async move {
            let guard = semaphore.acquire().await.unwrap();
            let event = rate.lock().spawn();
            tokio::time::sleep_until(event.time).await;
            println!("Starting {}", i);
            let sleep = thread_rng().gen_range(Duration::ZERO..Duration::from_secs(1));
            tokio::time::sleep(sleep).await;
            event.success.get_or_init(|| thread_rng().gen_bool(0.1));
            println!("Finished {}", i);
        }));
    }
    let start = Instant::now();
    for handle in handles {
        handle.await.unwrap();
    }
    println!("{:?}", start.elapsed());
}