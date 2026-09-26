//! The last readings, for the graphs.

use std::collections::VecDeque;

use crate::model::Sample;

/// Enough readings for a braille graph across a very wide screen.
pub const CAPACITY: usize = 1024;

/// Values from 0 to 1, oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Series(VecDeque<f64>);

impl Series {
    pub fn push(&mut self, value: f64) {
        if self.0.len() == CAPACITY {
            self.0.pop_front();
        }
        self.0.push_back(if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        });
    }

    /// The newest `count` values, oldest first.
    pub fn last(&self, count: usize) -> Vec<f64> {
        let skip = self.0.len().saturating_sub(count);
        self.0.iter().skip(skip).copied().collect()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    pub cpu: Series,
    pub memory: Series,
    pub swap: Series,
}

impl History {
    pub fn push(&mut self, sample: &Sample) {
        self.cpu.push(sample.cpu.total.busy());
        self.memory.push(sample.memory.used_ratio());
        self.swap.push(sample.swap.used_ratio());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_newest_values() {
        let mut series = Series::default();
        for value in 0..CAPACITY + 10 {
            series.push(value as f64 / 10_000.0);
        }
        assert_eq!(series.len(), CAPACITY);
        let last = series.last(3);
        assert_eq!(last.len(), 3);
        assert!((last[2] - (CAPACITY + 9) as f64 / 10_000.0).abs() < 1e-12);
        assert_eq!(series.last(5000).len(), CAPACITY);
    }

    #[test]
    fn keeps_values_between_zero_and_one() {
        let mut series = Series::default();
        series.push(1.5);
        series.push(-1.0);
        series.push(f64::NAN);
        assert_eq!(series.last(3), [1.0, 0.0, 0.0]);
    }
}
