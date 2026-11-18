//! Borrowed visible intervals, including explicit gaps before an initial value.
use crate::trace::{History, Logic};

pub struct Segment<'a> {
    pub start: u64,
    pub end: u64,
    pub value: Option<&'a [Logic]>,
}

pub fn visible(history: &History, start: u64, end: u64) -> Vec<Segment<'_>> {
    if start >= end {
        return Vec::new();
    }
    let mut output = Vec::new();
    let changes = history.changes();
    let mut index = changes.partition_point(|c| c.time <= start);
    let mut time = start;
    let mut value = history.value_at(start);
    while index < changes.len() && changes[index].time < end {
        let change = &changes[index];
        if time < change.time {
            output.push(Segment {
                start: time,
                end: change.time,
                value,
            });
        }
        time = change.time;
        value = Some(&change.value);
        index += 1;
    }
    if time < end {
        output.push(Segment {
            start: time,
            end,
            value,
        });
    }
    output
}
