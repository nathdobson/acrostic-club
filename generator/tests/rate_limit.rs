// Tests for `RateLimit`: prints the time at which each of 1000 tasks may start.

use acrostic_generator::util::clock::Clock;
use acrostic_generator::util::rate_limit::RateLimit;
use tokio::time::Instant;

#[tokio::test]
async fn test_rate_limit() {
    let mut rate = RateLimit::new(Clock::Real, 10, 10.0);
    for i in 0..1000 {
        println!("{:?} {:?}", i, Instant::now());
        tokio::time::sleep_until(rate.spawn()).await;
    }
}
