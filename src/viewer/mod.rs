//! Viewing state shared by the UI and its display-independent tests.
pub mod format;
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
    pub cursor: Option<u64>,
    pub viewport: Viewport,
    pub radix: Radix,
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
            cursor: None,
            viewport,
            radix: Radix::Binary,
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
        }
    }

    pub fn visible_signals(&self) -> Vec<usize> {
        let filter = self.filter.to_lowercase();
        self.trace
            .signals
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.path.split_last().is_some_and(|(name, scope)| {
                    scope == self.scope && name.to_lowercase().contains(&filter)
                })
            })
            .map(|(i, _)| i)
            .collect()
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
