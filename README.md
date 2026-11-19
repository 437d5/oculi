# Oculi

A Rust waveform exploration project, built while learning Rust. Explore completed
digital simulations in a native waveform viewer or print a command-line summary.

See the [roadmap](#roadmap) for planned waveform workflows and proposed milestones.

## Roadmap

This roadmap records the requested features. The milestone order is proposed;
it is not a release schedule. Unchecked items are planned, not implemented.
Interaction details will be agreed before implementing each feature.

Current foundation:

- [x] Digital VCD loading with hierarchy, aliases, and four-state values (0/1/X/Z).
- [x] Scope/signal search, waveform reordering, and expandable bus bits.
- [x] Multiple cursors with stable names/colors, active-cursor values, and zoom/pan.

### 1. Navigate and measure

- [ ] **Drag to zoom into a time window.** Select an interval directly in the
  waveform area and fit the viewport to it. Choose a gesture that works alongside
  the existing drag-to-pan and click-to-place-cursor controls.
- [ ] **Measure the time difference between cursors.** Show the elapsed time
  between two cursors in the trace's declared time units, or integer ticks when
  no timescale is declared. Proposed first version: two selectors choose the pair
  when several cursors exist, and a live readout shows `Δt = |t₂ - t₁|` as either
  cursor moves. Dragging markers, exact-time entry, and edge snapping can be
  considered later if needed.
- [ ] **Count signal edges between cursors.** Choose a signal and two cursors,
  then count edges within their interval. Define rising/falling edge options,
  endpoint inclusion, equal-time changes, and X/Z handling before implementation.

Cursor-pair selection should be shared by interval measurements and edge counting.

### 2. Select and organize signals

- [ ] **Select multiple signals and act on them together.** Support selecting
  several rows and applying bulk actions. Candidate actions include adding,
  removing, reordering, and changing display settings; browser selection and
  displayed-row selection need explicit behavior.
- [ ] **Change signal colors.** Choose colors for displayed signals, including
  selected rows together. Define how bus bits inherit or override a bus color.
- [ ] **Signal groups and group interaction.** Organize rows into named groups.
  Proposed actions include creating/renaming groups, expanding/collapsing them,
  moving groups, and adding/removing members. Reuse multiple-selection behavior.

Keep display choices separate from trace storage so aliases can be styled and
organized independently while continuing to share waveform data.

### 3. Save and reuse a debugging view

- [ ] **Save and restore sessions.** Retain the trace reference, signal order,
  expanded bus bits, groups, colors, cursors and measurement pair, viewport, and
  display settings. Identify signals by hierarchical path and compatible width
  rather than runtime indices, preserving distinct alias names. Use a versioned
  session format and resolve relative trace paths from the session file.
- [ ] **Reload an updated VCD.** Refresh a trace after rerunning the simulation
  while retaining compatible rows and display settings. Report missing or
  incompatible signals and clamp times if the new trace is shorter. Failed
  reloads preserve the current view; failed saves preserve an existing session.

### 4. Navigate events and find values

- [ ] **Next/previous transition navigation.** Move the active cursor to the
  next or previous change on a selected signal, with rising/falling edge options
  for scalar signals. Share edge definitions with edge counting, including X/Z,
  equal-time changes, and behavior at the first/last event.

- [ ] **Find a value on a waveform.** Search a selected signal and navigate to
  matching times. Define forward/backward search, value entry formats, and
  whether X/Z are literal values or wildcards.

### 5. Keep larger traces responsive

- [ ] **Measure representative traces.** Record file size, signal/change counts,
  load time, peak memory, and navigation latency. Include dense clocks, wide
  buses, deep hierarchy, and long quiet intervals. Start measuring early and use
  the results to prioritize performance work.
- [ ] **Background loading with progress and cancellation.** Keep the window
  responsive and install a replacement trace only after loading succeeds.
- [ ] **Improve storage and rendering where measurements justify it.** Reduce
  memory use and avoid unnecessary per-frame work while preserving exact values,
  timestamps, and visible activity from narrow pulses.

Establish a memory and responsiveness baseline before opening several full traces.

### 6. Evaluate expressions and explore multiple traces

- [ ] **Advanced: evaluate expressions involving signals.** Define supported
  operators, width/signedness rules, four-state behavior, and how results appear
  before building an evaluator. Start with expressions within one trace.
- [ ] **Several VCD tabs.** Consider opening multiple traces in separate tabs,
  each retaining its own selected signals, groups, colors, cursors, and viewport.
  Measure memory use before committing to this feature: each trace is currently
  loaded completely into memory. Cross-trace comparison is a separate decision.

Value search provides a smaller first step before expression evaluation.

### Longer-term ideas

Candidates to revisit after using the earlier milestones on real debugging tasks;
these are not committed milestones:

- **FST support**, if the simulations produce it. Evaluate a format adapter
  together with the storage measurements; changing file formats alone does not
  solve memory use if every sample is expanded on load.
- **Waveform image export** for sharing selected rows and a time interval.
- **Compare simulation runs** after independent tabs work well, with explicit
  time alignment and handling of different timescales or missing signals.
- **Protocol decoding**, such as UART or SPI, when a concrete debugging task
  justifies it.

First proposed increment: drag-to-zoom, followed by the agreed cursor interactions
and edge counting. Feature priorities can be changed after trying each increment.

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
Use **Find scope…** above the hierarchy to fuzzy-match full scope paths: matching
is case-insensitive and letters can be skipped (`td` finds `tb.dut`). Results are
ranked, with exact scope names first. Use ↑/↓ and Enter, or click a result, to
select its scope. **Clear** or Escape restores the tree.

**Find signal…** uses the same fuzzy matching and ranking within the selected
scope (`ct` finds `count[3:0]`). Use ↑/↓ to highlight a signal and Enter to toggle
its waveform, or use its checkbox. **Clear** or Escape restores the scope's full
signal list without removing displayed waveforms. The two searches are independent.

- Click a waveform to place the first cursor or move the active cursor.
  **Ctrl+click** creates a new cursor at that point. **Add cursor** creates another
  at the active visible cursor's time, or the viewport centre if none is visible;
  click the waveform to position it. Use the **Cursor A/B/…** selector to switch
  the active cursor and **Remove cursor** to remove it. Names and colors stay
  stable, and all visible cursors are drawn with ruler labels. The value column
  shows the active cursor's values, including expanded bus bits.
- Zoom in / Zoom out buttons navigate time. `+` (or `=`), `-`, and `F` zoom in,
  zoom out, and Fit trace when a text field is not focused. Buttons and shortcuts
  zoom around the active visible cursor, or the centre of the viewport otherwise.
  Scrolling over a waveform zooms around the mouse.
- Drag a waveform horizontally to pan. Scroll over names to move through rows.
- Drag the dotted handle beside a signal name onto another name row to reorder
  the displayed signals. Right-click a name for Move up / Move down. Releasing
  a dragged handle outside the name rows leaves the order unchanged.
- Choose Binary or Hex for buses; hover truncated values for full text.
- Click **+** beside a displayed bus name to expand it into individual bit
  waveforms, or right-click its name and choose **Expand bits**. The bus summary
  stays above its bits; click **−** to collapse them. Each bit has its
  own cursor value, high-state shading, and X/Z display. Bit names follow the VCD
  declaration, including ascending or offset ranges; a bus without a declared
  range uses width-1 through zero. Dragging the bus handle moves its bits with it.
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
imports. `Session` owns a trace plus selection, scope, cursors, and viewport state.
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
