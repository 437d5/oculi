//! Display rows borrow complete signals or individual positions in bus samples.
use crate::trace::{Logic, Trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WaveRow {
    pub signal: usize,
    /// Position in the stored MSB-first value, not the source-level bit index.
    pub bit: Option<usize>,
}

impl WaveRow {
    pub fn project<'a>(&self, value: &'a [Logic]) -> Option<&'a [Logic]> {
        match self.bit {
            Some(position) => value.get(position..position.checked_add(1)?),
            None => Some(value),
        }
    }

    pub fn label(&self, trace: &Trace) -> String {
        let signal = &trace.signals[self.signal];
        self.bit
            .and_then(|position| signal.bit_name(position))
            .unwrap_or_else(|| signal.path.join("."))
    }
}
