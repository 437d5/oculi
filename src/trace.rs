//! Owned waveform data, independent of the file format and user interface.

/// A single digital bit, including unknown and high-impedance states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Logic {
    Zero,
    One,
    X,
    Z,
}

/// A value at an integer tick. Bits are ordered most significant first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub time: u64,
    pub value: Vec<Logic>,
}

/// Initial value and subsequent changes, with adjacent duplicates removed.
#[derive(Debug, Default)]
pub struct History {
    changes: Vec<Change>,
}

impl History {
    /// Borrow the last value recorded at or before `time` (no value is invented).
    pub fn value_at(&self, time: u64) -> Option<&[Logic]> {
        let after = self.changes.partition_point(|change| change.time <= time);
        after
            .checked_sub(1)
            .map(|index| self.changes[index].value.as_slice())
    }

    /// Borrow the samples without copying their values.
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// Count actual value changes, excluding the first observed value.
    pub fn transition_count(&self) -> usize {
        self.changes.len().saturating_sub(1)
    }

    // Loaders validate time ordering and width before recording a sample.
    pub(crate) fn record(&mut self, time: u64, value: Vec<Logic>) {
        if self.changes.last().is_none_or(|last| last.value != value) {
            self.changes.push(Change { time, value });
        }
    }
}

/// A declaration. Aliases share a history index but retain their own paths.
#[derive(Debug)]
pub struct Signal {
    /// Separate scope/name components preserve escaped names containing dots.
    pub path: Vec<String>,
    pub width: u32,
    /// Declared left/right bit indices; None uses width-1 down to zero.
    pub bit_range: Option<(i32, i32)>,
    /// Index into the owning trace's histories.
    pub history: usize,
}

impl Signal {
    /// Label a recorded bit position, where position zero is the leftmost bit.
    pub fn bit_name(&self, position: usize) -> Option<String> {
        let position = i64::try_from(position).ok()?;
        if position >= i64::from(self.width) {
            return None;
        }
        let label = self.path.join(".");
        let (base, index) = if let Some((left, right)) = self.bit_range {
            let suffix = format!("[{left}:{right}]");
            let base = label.strip_suffix(&suffix).unwrap_or(&label);
            let index = i64::from(left) + if left <= right { position } else { -position };
            (base, index)
        } else {
            (label.as_str(), i64::from(self.width) - 1 - position)
        };
        Some(format!("{base}[{index}]"))
    }
}

/// Time units used by trace formats, independent of a parser library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
    Picoseconds,
    Femtoseconds,
}

/// One trace tick equals `magnitude` units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timescale {
    pub magnitude: u32,
    pub unit: TimeUnit,
}

/// The trace owns all its data; readers can borrow it without retaining the file.
#[derive(Debug, Default)]
pub struct Trace {
    /// Last recorded timestamp, including quiet time after the last change.
    pub end_time: u64,
    pub timescale: Option<Timescale>,
    /// Signals in declaration order for deterministic presentation.
    pub signals: Vec<Signal>,
    pub histories: Vec<History>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_lookup_borrows_last_value_at_or_before_time() {
        let mut history = History::default();
        assert_eq!(history.value_at(0), None);
        history.record(5, vec![Logic::Zero]);
        history.record(5, vec![Logic::One]);
        history.record(9, vec![Logic::X]);
        assert_eq!(history.value_at(4), None);
        assert_eq!(history.value_at(5), Some(&[Logic::One][..]));
        assert_eq!(history.value_at(8), Some(&[Logic::One][..]));
        assert_eq!(history.value_at(9), Some(&[Logic::X][..]));
        assert_eq!(history.value_at(u64::MAX), Some(&[Logic::X][..]));
    }

    #[test]
    fn counts_value_transitions_without_counting_initial_or_duplicate_samples() {
        let mut history = History::default();
        assert_eq!(history.transition_count(), 0);
        history.record(0, vec![Logic::X]);
        assert_eq!(history.transition_count(), 0);
        history.record(1, vec![Logic::X]);
        history.record(2, vec![Logic::Zero]);
        history.record(3, vec![Logic::Zero]);
        history.record(4, vec![Logic::Z]);
        assert_eq!(history.transition_count(), 2);
        assert_eq!(history.changes().len(), 3);
        assert_eq!(history.changes()[1].time, 2);
    }

    #[test]
    fn aliases_borrow_the_same_history() {
        let mut history = History::default();
        history.record(0, vec![Logic::Zero]);
        history.record(5, vec![Logic::One]);
        let trace = Trace {
            end_time: 5,
            timescale: Some(Timescale {
                magnitude: 1,
                unit: TimeUnit::Nanoseconds,
            }),
            signals: vec![
                Signal {
                    path: vec!["tb".into(), "clk".into()],
                    width: 1,
                    bit_range: None,
                    history: 0,
                },
                Signal {
                    path: vec!["tb".into(), "dut".into(), "clk".into()],
                    width: 1,
                    bit_range: None,
                    history: 0,
                },
            ],
            histories: vec![history],
        };
        let first = &trace.histories[trace.signals[0].history];
        let alias = &trace.histories[trace.signals[1].history];
        assert!(std::ptr::eq(first, alias));
        assert_eq!(alias.transition_count(), 1);
    }
}
