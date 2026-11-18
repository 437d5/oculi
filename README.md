# Oculi

A Rust waveform exploration project, built while learning Rust. Explore completed
digital simulations in a native waveform viewer or print a command-line summary.

See the [waveform feature roadmap](docs/ROADMAP.md) for proposed next steps and a
worksheet for choosing the workflows you want from your existing tools.

## Native waveform viewer

```sh
cargo run --features gui --bin oculi-gui -- build/counter.vcd
# Or start without a file and use Open VCD:
cargo run --features gui --bin oculi-gui
```

Generate `build/counter.vcd` with [the counter example](examples/counter/README.md),
or open the small `examples/counter.vcd` fixture. GUI compilation is optional;
ordinary `cargo run -- <file>` still runs the summary CLI.

Layout A has hierarchy and signal selection on the left, aligned names, cursor
values, and waveforms on the right. Select a scope, filter its signals, and check
the ones to display. Alias names are independently selectable and share storage.
Click scope names to select them; use the separate arrows to expand/collapse.
Changing scopes clears the old filter so it cannot hide a different scope's signals.

- Click a waveform to place cursor A and inspect values.
- Zoom in / Zoom out buttons navigate time. `+` (or `=`), `-`, and `F` zoom in,
  zoom out, and Fit trace when a text field is not focused. Buttons and shortcuts
  zoom around the visible time cursor, or the centre of the viewport otherwise.
  Scrolling over a waveform zooms around the mouse.
- Drag a waveform horizontally to pan. Scroll over names to move through rows.
- Drag the dotted handle beside a signal name onto another name row to reorder
  the displayed signals. Right-click a name for Move up / Move down. Releasing
  a dragged handle outside the name rows leaves the order unchanged.
- Choose Binary or Hex for buses; hover truncated values for full text.
- Light theme / Dark theme changes appearance; the initial theme follows the system.
- A failed Open VCD preserves the previous trace. Cancelling the file dialog does nothing.

Labels use the declared time unit and magnitude: `5000 ps` means `5 ns`. No
timescale means integer ticks. A cursor uses the last recorded value at or before
its timestamp, including the last value at equal timestamps. Before the first
sample the value is unavailable. Final values extend to the last timestamp,
including quiet time. Zero-duration traces have no interval to draw.

This version loads small traces synchronously and holds them in memory; large
files can pause the window. Only one trace is open at a time; sessions are not saved.
The GUI uses eframe/egui with OpenGL and X11/Wayland, rfd for file dialogs, and
egui_extras for the saved SVG logo. It requires a desktop display and OpenGL.
On Linux, dialogs use an installed XDG desktop portal backend, with Zenity as a
fallback. Passing a startup file avoids needing to open a dialog.

The GUI prefers an installed regular JetBrains Mono (including its Nerd Font
Mono variant), with egui's bundled fonts retained as fallbacks. It checks common
user/system font paths; an unavailable font does not prevent startup. No local
font file is copied into this repository. Waveform labels use regular 14-point
text and ruler labels use 12-point text. Ruler ticks are distinct integer times,
and labels are spaced using their measured width.

## Run

Install a current stable Rust toolchain, then from this directory:

```sh
cargo run -- examples/counter.vcd
cargo run -- --help
```

Cargo downloads the `vcd` parser on the first build. Everything after `--` is passed
to Oculi rather than Cargo.

The example produces tab-separated output:

```text
Timescale: 1 ns
Signal          Bits    Transitions
tb.clk          1       5
tb.count[3:0]   4       3
tb.dut.clk      1       5
```

Each count excludes the first observed value and repeated assignments of the same
value. X (unknown) and Z (high impedance) are preserved as distinct values. The two
clock names are aliases: they share storage and show the same transition count.
Changes at the same timestamp retain their input order.

For your own SystemVerilog simulation, generate a VCD with your simulator's tracing
option or with `$dumpfile` and `$dumpvars` in the testbench, then pass its path to Oculi.
Oculi reads traces; it does not execute SystemVerilog.

For a runnable SystemVerilog example, see [the counter and directed testbench](examples/counter/README.md).
They generate a fresh VCD with Verilator and check reset, counting, pause, and wraparound.

## Supported subset

- Digital scalars and binary vectors, including X and Z, scopes, and aliases.
- Integer tick timestamps and an optional declared timescale. No unit is invented
  when the timescale is absent.
- `$dumpvars`, `$dumpall`, comments, and the usual date/version header metadata.
- Declared ranges are included in signal names, for example `count[3:0]`.

Real values, strings, event declarations, and `$dumpoff`/`$dumpon` are rejected with
an error. Recording gaps require a richer model before they can be shown honestly.
Malformed syntax, inconsistent alias widths, undeclared identifiers, over-wide
vectors, and backward timestamps are errors, not silently discarded data.

The complete trace is held in memory using one enum value per bit. This is a
starting point for small traces, not yet storage optimized for large simulations.
FST, source navigation, and live simulator integration are future work.

## Read the code together

1. `src/trace.rs`: start with `Logic`, then `Change`, `History`, `Signal`, and `Trace`.
   An enum expresses the four possible bit states. `Trace` owns its vectors; no
   garbage collector or borrowed file buffer is needed to keep the trace alive.
2. `examples/counter.vcd`: compare the text with the summary. `#5` is tick 5, `!`
   identifies the clock, and the two clock declarations reuse that identifier.
3. `src/vcd.rs`: the parser dependency handles syntax. Oculi resolves identifiers,
   validates supported data, and converts it into its own model. `match` handles
   each command; `?` propagates a failed `Result` to the caller.
4. `src/main.rs`: a thin executable opens a file with `BufReader` and borrows the
   trace to print it. `args_os` and `PathBuf` preserve non-UTF-8 filesystem paths.

`&self` means a method borrows an existing value. `&[Change]` is a read-only view
of a sequence; calling `changes()` does not clone its data. `&mut self` is an
exclusive borrow that permits mutation, as used internally when loading samples.

The library and executables are separate Cargo targets. The GUI uses the
library without using the CLI, and another format loader can produce the same
trace model. Parser-library types stay inside the VCD adapter. Introduce traits
when a second implementation needs a shared behavior; no plugin framework yet.

For the GUI, start with `src/viewer/`: ordinary data and calculations with no GUI
imports. `Session` owns a trace plus selection, scope, cursor, and viewport state.
Then read `src/gui/mod.rs` and `browser.rs`: eframe calls an `App` implementation
to describe the interface, and button responses tell us when to change state.
Finally, `src/gui/waves.rs` maps integer ticks to screen coordinates and paints
the rows. `History::value_at` returns `Option<&[Logic]>`: either no available value,
or a borrowed slice of existing bits, without copying waveform data.

## Check changes

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo clippy --all-targets --features gui -- -D warnings
cargo test --features gui
```

Tests exercise the real parser and executable, including invalid input and
non-UTF-8 paths on Unix. See `docs/superpowers/specs/` and
`docs/superpowers/plans/` for the agreed design and implementation plan.
