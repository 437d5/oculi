//! Trace storage and file loading for the Oculi waveform viewer.

pub mod trace;
pub mod vcd;
pub mod viewer;

#[cfg(feature = "gui")]
pub mod gui;
