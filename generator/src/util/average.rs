use std::time::Instant;

pub struct RunningAverage {
    average: f64,
    last_x: f64,
    param: f64,
}

impl RunningAverage {
    pub fn new(param: f64) -> Self {
        RunningAverage { average: f64::NAN, last_x: f64::NEG_INFINITY, param }
    }
    const ANALYTIC_BOUNDARY: f64 = 0.1;
    fn new_weight(dx: f64) -> f64 {
        if dx < Self::ANALYTIC_BOUNDARY {
            Self::new_weight_poly(dx)
        } else {
            Self::new_weight_analytic(dx)
        }
    }
    fn new_weight_analytic(dx: f64) -> f64 {
        (1.0 - (-dx).exp()) / dx
    }
    fn new_weight_poly(dx: f64) -> f64 {
        1.0 - dx / 2.0 + dx * dx / 6.0 - dx * dx * dx / 24.0
    }
    fn old_weight(dx: f64) -> f64 {
        (-dx).exp()
    }

    pub fn add(&mut self, x: f64, y: f64) {
        if self.last_x == f64::NEG_INFINITY {
            self.last_x = x;
            self.average = y;
            return;
        }

        assert!(x >= self.last_x);
        let dx = (x - self.last_x) / self.param;
        self.average = y * Self::new_weight(dx) / self.param + self.average * Self::old_weight(dx);
        self.last_x = x;
    }
    pub fn average(&self) -> f64 {
        self.average
    }
    pub fn x(&self) -> f64 {
        self.last_x
    }
}

#[test]
fn test_running_average() {
    let mut average = RunningAverage::new(1.0);
    average.add(1.0, 1.0);
    assert_eq!(average.average, 1.0);
    average.add(1.0, 1.0);
    assert_eq!(average.average, 2.0);
    average.add(1.000001, 1.0);
    assert_eq!(average.average, 2.999997500001167);
    average.add(2.0, 1.0);
    assert_eq!(average.average, 1.7357593305238646);
    average.add(3.0, 1.0);
    average.add(4.0, 1.0);
    average.add(5.0, 1.0);
    average.add(6.0, 1.0);
    average.add(7.0, 1.0);
    assert_eq!(average.average, 1.0049575073731525);
    for param in 1..10 {
        let mut average = RunningAverage::new(param as f64);
        for i in [1, 2, 3, 4, 5] {
            average.add(i as f64, i as f64);
            println!("{:?}", average.average());
        }
    }
}


#[test]
fn test_floating() {
    // assert_eq!(RunningAverage::new_weight(1.0)
    assert!(RunningAverage::new_weight_poly(RunningAverage::ANALYTIC_BOUNDARY)
        - RunningAverage::new_weight_analytic(RunningAverage::ANALYTIC_BOUNDARY) < 0.001);
}