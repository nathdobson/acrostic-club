// Tests for `RateLimit`: prints the time at which each of 1000 tasks may start.

use acrostic_generator::util::clock::Clock;
use acrostic_generator::util::rate_limit::RateLimit;
use tokio::time::Instant;

#[tokio::test]
async fn a_rate_limit_schedules_a_thousand_tasks() {
    let mut rate_limit = RateLimit::new(Clock::Real, 10, 10.0);

    for i in 0..1000 {
        let now = Instant::now();
        println!("{i:?} {now:?}");
        tokio::time::sleep_until(rate_limit.spawn()).await;
    }
}
