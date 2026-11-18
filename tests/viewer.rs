use oculi::{
    trace::{Logic::*, TimeUnit, Timescale},
    viewer::{
        Session,
        format::{Radix, format_time, format_value},
        segments,
        viewport::Viewport,
    },
};

fn sample() -> oculi::trace::Trace {
    oculi::vcd::load(
        &b"$var wire 1 ! root $end
$scope module tb $end
$var wire 1 ! clk $end
$var wire 4 \" data $end
$scope module dut $end
$var wire 1 ! alias $end
$upscope $end
$upscope $end
$enddefinitions $end
#5
0!
b0011 \"
#10
1!
#10
x!
#100
"[..],
    )
    .unwrap()
}

#[test]
fn selection_shares_histories_and_filters_by_scope() {
    let mut s = Session::new(sample());
    assert_eq!(s.selected, [0, 2]);
    assert_eq!(s.visible_signals(), [0]);
    s.scope = vec!["tb".into()];
    assert_eq!(s.visible_signals(), [1, 2]);
    s.filter = "DAT".into();
    assert_eq!(s.visible_signals(), [2]);
    s.set_selected(3, true);
    s.set_selected(3, true);
    s.set_selected(999, true);
    assert_eq!(s.selected, [0, 2, 3]);
    s.set_selected(0, false);
    assert_eq!(s.selected, [2, 3]);
    assert_eq!(s.trace.histories.len(), 2);
    s.cursor = Some(12);
    let reset = Session::new(sample());
    assert_eq!(reset.cursor, None);
    assert!(reset.filter.is_empty());
    assert_eq!(reset.viewport, Viewport::fit(100));
}

#[test]
fn initially_limits_selection_to_eight_unique_histories() {
    let mut input = String::new();
    for n in 0..10 {
        input.push_str(&format!("$var wire 1 {} s{n} $end\n", (b'!' + n) as char));
    }
    input.push_str("$enddefinitions $end\n");
    let s = Session::new(oculi::vcd::load(input.as_bytes()).unwrap());
    assert_eq!(s.selected, [0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn waveform_reordering_changes_only_display_order() {
    let mut s = Session::new(sample());
    s.set_selected(3, true);
    s.move_selected(0, 2);
    assert_eq!(s.selected, [2, 3, 0]);
    s.move_selected(2, 0);
    assert_eq!(s.selected, [0, 2, 3]);
    s.move_selected(0, 0);
    s.move_selected(0, 999);
    s.move_selected(999, 0);
    assert_eq!(s.selected, [0, 2, 3]);
    s.move_selected(2, 1);
    s.set_selected(0, false);
    s.set_selected(1, true);
    assert_eq!(s.selected, [3, 2, 1]);
    assert_eq!(s.trace.signals[0].history, s.trace.signals[3].history);
    assert_eq!(s.trace.histories.len(), 2);
}

#[test]
fn ruler_ticks_are_distinct_integer_times_without_overflow() {
    use oculi::viewer::ruler::ticks;
    assert_eq!(ticks(0, 100, 6), [0, 20, 40, 60, 80, 100]);
    assert_eq!(ticks(0, 1, 10), [0, 1]);
    assert_eq!(ticks(25, 75, 6), [30, 40, 50, 60, 70]);
    assert_eq!(
        ticks(101, 104, 2),
        [101, 104],
        "narrow shifted windows must retain a ruler"
    );
    assert_eq!(ticks(u64::MAX - 1, u64::MAX, 10), [u64::MAX - 1, u64::MAX]);
    assert!(ticks(5, 4, 10).is_empty());
    assert!(ticks(0, 100, 0).is_empty());
    let wide = ticks(0, u64::MAX, 10);
    assert!(!wide.is_empty());
    assert!(wide.len() <= 10);
    assert!(wide.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn formats_arbitrary_width_four_state_values() {
    assert_eq!(format_value(&[Zero, Zero, One, One], Radix::Binary), "0011");
    assert_eq!(format_value(&[Zero, Zero, One, One], Radix::Hex), "3");
    assert_eq!(format_value(&[One, Zero, One, Zero, One], Radix::Hex), "15");
    assert_eq!(format_value(&[Z, Z, Z, Z], Radix::Hex), "Z");
    assert_eq!(format_value(&[Zero, Z, One, X], Radix::Hex), "X");
    assert_eq!(format_value(&[One; 132], Radix::Hex), "F".repeat(33));
    assert_eq!(
        format_time(
            5,
            Some(Timescale {
                magnitude: 10,
                unit: TimeUnit::Picoseconds
            })
        ),
        "50 ps"
    );
    assert_eq!(format_time(5, None), "5 ticks");
    assert_eq!(
        format_time(
            u64::MAX,
            Some(Timescale {
                magnitude: 100,
                unit: TimeUnit::Seconds
            })
        ),
        "1844674407370955161500 s"
    );
}

#[test]
fn navigation_clamps_and_preserves_high_offset_precision() {
    let mut v = Viewport::fit(100);
    assert_eq!((v.tick_at(0.0), v.tick_at(1.0)), (0, 100));
    v.zoom(0.5, 50, 100);
    assert_eq!((v.start, v.end), (25, 75));
    v.pan(-1000, 100);
    assert_eq!((v.start, v.end), (0, 50));
    v.pan(i128::MAX, 100);
    assert_eq!((v.start, v.end), (50, 100));
    let old = v;
    v.zoom(f64::NAN, 0, 100);
    assert_eq!(old, v);
    assert!(Viewport::fit(0).end > 0);
    let high = Viewport {
        start: u64::MAX - 100,
        end: u64::MAX,
    };
    assert_eq!(high.tick_at(0.5), u64::MAX - 50);
    assert_eq!(high.fraction_at(u64::MAX - 50), 0.5);
    assert_eq!(high.tick_at(f64::NAN), high.start);
}

#[test]
fn visible_segments_preserve_missing_values_equal_times_and_quiet_tail() {
    let trace = sample();
    let parts = segments::visible(&trace.histories[0], 0, trace.end_time);
    assert_eq!(
        parts
            .iter()
            .map(|s| (s.start, s.end, s.value.map(|v| v[0])))
            .collect::<Vec<_>>(),
        [(0, 5, None), (5, 10, Some(Zero)), (10, 100, Some(X))]
    );
    let clipped = segments::visible(&trace.histories[0], 7, 12);
    assert_eq!(
        clipped
            .iter()
            .map(|s| (s.start, s.end, s.value.map(|v| v[0])))
            .collect::<Vec<_>>(),
        [(7, 10, Some(Zero)), (10, 12, Some(X))]
    );
    assert!(segments::visible(&trace.histories[0], 10, 10).is_empty());
    let empty = oculi::trace::History::default();
    assert_eq!(segments::visible(&empty, 0, 10)[0].value, None);
    let bus = segments::visible(&trace.histories[1], 5, 100);
    assert_eq!((bus[0].start, bus[0].end), (5, 100));
}
