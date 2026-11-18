//! Text formatting without any dependency on a GUI library.
use crate::trace::{Logic, TimeUnit, Timescale};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radix {
    Binary,
    Hex,
}

pub fn format_value(value: &[Logic], radix: Radix) -> String {
    if value.is_empty() {
        return "—".into();
    }
    if radix == Radix::Binary {
        return value
            .iter()
            .map(|bit| match bit {
                Logic::Zero => '0',
                Logic::One => '1',
                Logic::X => 'X',
                Logic::Z => 'Z',
            })
            .collect();
    }
    // Chunk from the most significant end, allowing a short leading group.
    let first = if value.len().is_multiple_of(4) {
        4
    } else {
        value.len() % 4
    };
    let mut output = String::new();
    let mut offset = 0;
    while offset < value.len() {
        let size = if offset == 0 { first } else { 4 };
        let group = &value[offset..offset + size];
        let digit = if group.iter().all(|&b| b == Logic::Z) {
            'Z'
        } else if group.iter().any(|&b| matches!(b, Logic::X | Logic::Z)) {
            'X'
        } else {
            let n = group
                .iter()
                .fold(0u8, |n, &b| n * 2 + u8::from(b == Logic::One));
            char::from(b"0123456789ABCDEF"[n as usize])
        };
        output.push(digit);
        offset += size;
    }
    output
}

pub fn format_time(ticks: u64, timescale: Option<Timescale>) -> String {
    let Some(scale) = timescale else {
        return format!("{ticks} ticks");
    };
    let unit = match scale.unit {
        TimeUnit::Seconds => "s",
        TimeUnit::Milliseconds => "ms",
        TimeUnit::Microseconds => "us",
        TimeUnit::Nanoseconds => "ns",
        TimeUnit::Picoseconds => "ps",
        TimeUnit::Femtoseconds => "fs",
    };
    format!("{} {unit}", u128::from(ticks) * u128::from(scale.magnitude))
}
