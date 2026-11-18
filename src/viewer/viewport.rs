//! Integer time bounds with floating-point conversion only at the screen edge.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub start: u64,
    pub end: u64,
}

impl Viewport {
    pub fn fit(end_time: u64) -> Self {
        Self {
            start: 0,
            end: end_time.max(1),
        }
    }

    pub fn span(&self) -> u64 {
        self.end.saturating_sub(self.start).max(1)
    }

    pub fn tick_at(&self, fraction: f64) -> u64 {
        if !fraction.is_finite() {
            return self.start;
        }
        if fraction >= 1.0 {
            return self.end;
        }
        let offset = (self.span() as f64 * fraction.clamp(0.0, 1.0)).round() as u64;
        self.start.saturating_add(offset).min(self.end)
    }

    pub fn fraction_at(&self, tick: u64) -> f64 {
        // Subtract while still integer: short windows at high offsets stay exact.
        if tick < self.start {
            -((self.start - tick) as f64) / self.span() as f64
        } else {
            (tick - self.start) as f64 / self.span() as f64
        }
    }

    pub fn pan(&mut self, delta: i128, end_time: u64) {
        let bound = end_time.max(1);
        let span = self.span().min(bound);
        let start = i128::from(self.start)
            .saturating_add(delta)
            .clamp(0, i128::from(bound - span)) as u64;
        self.start = start;
        self.end = start + span;
    }

    /// Factors below one zoom in, preserving the anchor's screen position.
    pub fn zoom(&mut self, factor: f64, anchor: u64, end_time: u64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let bound = end_time.max(1);
        let fraction = self.fraction_at(anchor).clamp(0.0, 1.0);
        let span = ((self.span() as f64 * factor).round() as u64).clamp(1, bound);
        let before = (span as f64 * fraction).round() as u64;
        self.start = anchor.saturating_sub(before).min(bound - span);
        self.end = self.start + span;
    }
}
