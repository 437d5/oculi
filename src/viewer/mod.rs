//! Viewing state shared by the UI and its display-independent tests.
pub mod cursors;
pub mod format;
pub mod hierarchy;
pub mod rows;
pub mod ruler;
pub mod segments;
pub mod viewport;

use crate::trace::Trace;
use format::Radix;
use viewport::Viewport;

/// Owns a loaded trace and the user's current view of it.
pub struct Session {
    pub trace: Trace,
    pub selected: Vec<usize>,
    pub scope: Vec<String>,
    pub filter: String,
    pub signal_match: usize,
    pub hierarchy_query: String,
    pub hierarchy_match: usize,
    pub cursors: cursors::Cursors,
    pub viewport: Viewport,
    pub radix: Radix,
    expanded_buses: std::collections::HashSet<usize>,
}

impl Session {
    /// Move a displayed row, leaving trace declarations and shared histories intact.
    pub fn move_selected(&mut self, from: usize, to: usize) {
        if from >= self.selected.len() || to >= self.selected.len() || from == to {
            return;
        }
        let signal = self.selected.remove(from);
        self.selected.insert(to, signal);
    }

    /// A filter belongs to its current scope; do not silently apply it elsewhere.
    pub fn select_scope(&mut self, scope: Vec<String>) {
        if self.scope != scope {
            self.scope = scope;
            self.filter.clear();
            self.signal_match = 0;
        }
    }

    pub fn new(trace: Trace) -> Self {
        let mut seen = std::collections::HashSet::new();
        let selected = trace
            .signals
            .iter()
            .enumerate()
            .filter(|(_, signal)| seen.insert(signal.history))
            .take(8)
            .map(|(index, _)| index)
            .collect();
        let scope = trace
            .signals
            .first()
            .map(|s| s.path[..s.path.len().saturating_sub(1)].to_vec())
            .unwrap_or_default();
        let viewport = Viewport::fit(trace.end_time);
        Self {
            trace,
            selected,
            scope,
            filter: String::new(),
            signal_match: 0,
            hierarchy_query: String::new(),
            hierarchy_match: 0,
            cursors: cursors::Cursors::default(),
            viewport,
            radix: Radix::Binary,
            expanded_buses: std::collections::HashSet::new(),
        }
    }

    pub fn set_selected(&mut self, index: usize, selected: bool) {
        if index >= self.trace.signals.len() {
            return;
        }
        if selected && !self.selected.contains(&index) {
            self.selected.push(index);
        }
        if !selected {
            self.selected.retain(|&i| i != index);
            self.expanded_buses.remove(&index);
        }
    }

    pub fn add_cursor(&mut self, time: u64) -> cursors::CursorId {
        self.cursors.add(time.min(self.trace.end_time))
    }

    pub fn place_cursor(&mut self, time: u64) -> cursors::CursorId {
        self.cursors.place(time.min(self.trace.end_time))
    }

    pub fn is_bus_expanded(&self, index: usize) -> bool {
        self.expanded_buses.contains(&index)
    }

    pub fn set_bus_expanded(&mut self, index: usize, expanded: bool) {
        if !expanded {
            self.expanded_buses.remove(&index);
        } else if self.selected.contains(&index)
            && self
                .trace
                .signals
                .get(index)
                .is_some_and(|signal| signal.width > 1)
        {
            self.expanded_buses.insert(index);
        }
    }

    /// Expanded buses keep their summary row and move together with their bit rows.
    pub fn wave_rows(&self) -> Vec<rows::WaveRow> {
        let mut rows = Vec::new();
        for &index in &self.selected {
            let Some(signal) = self.trace.signals.get(index) else {
                continue;
            };
            rows.push(rows::WaveRow {
                signal: index,
                bit: None,
            });
            if self.is_bus_expanded(index) {
                rows.extend((0..signal.width as usize).map(|position| rows::WaveRow {
                    signal: index,
                    bit: Some(position),
                }));
            }
        }
        rows
    }

    pub fn visible_signals(&self) -> Vec<usize> {
        let candidates = self
            .trace
            .signals
            .iter()
            .enumerate()
            .filter_map(|(index, signal)| {
                let (name, scope) = signal.path.split_last()?;
                (scope == self.scope).then_some((index, std::slice::from_ref(name)))
            });
        hierarchy::ranked(candidates, &self.filter)
    }

    /// Sorted distinct scope paths, including ancestors and the root.
    pub fn scopes(&self) -> Vec<Vec<String>> {
        let mut scopes = std::collections::BTreeSet::new();
        scopes.insert(Vec::new());
        for signal in &self.trace.signals {
            for depth in 1..signal.path.len() {
                scopes.insert(signal.path[..depth].to_vec());
            }
        }
        scopes.into_iter().collect()
    }
}
