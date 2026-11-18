use oculi::{
    trace::Logic::{One, X, Z, Zero},
    viewer::{Session, rows::WaveRow},
};

fn ranged_bus(range: &str) -> Session {
    let input = format!(
        "$scope module tb $end\n$var wire 4 ! data {range} $end\n\
         $upscope $end\n$enddefinitions $end\n#5\nb10xz !\n#10\nb0101 !\n#100\n"
    );
    Session::new(oculi::vcd::load(input.as_bytes()).unwrap())
}

#[test]
fn bit_rows_keep_declared_indices_and_borrow_the_correct_four_state_values() {
    for (range, labels) in [
        (
            "[7:4]",
            ["tb.data[7]", "tb.data[6]", "tb.data[5]", "tb.data[4]"],
        ),
        (
            "[0:3]",
            ["tb.data[0]", "tb.data[1]", "tb.data[2]", "tb.data[3]"],
        ),
        (
            "[-1:-4]",
            ["tb.data[-1]", "tb.data[-2]", "tb.data[-3]", "tb.data[-4]"],
        ),
        ("", ["tb.data[3]", "tb.data[2]", "tb.data[1]", "tb.data[0]"]),
        (
            "[5]",
            [
                "tb.data[5][3]",
                "tb.data[5][2]",
                "tb.data[5][1]",
                "tb.data[5][0]",
            ],
        ),
    ] {
        let mut session = ranged_bus(range);
        let trace = &session.trace;
        let history = &trace.histories[0];
        for (position, expected) in [One, Zero, X, Z].into_iter().enumerate() {
            let row = WaveRow {
                signal: 0,
                bit: Some(position),
            };
            assert_eq!(row.label(trace), labels[position]);
            assert_eq!(
                history.value_at(4).and_then(|value| row.project(value)),
                None
            );
            let source = history.value_at(5).unwrap();
            let projected = row.project(source).unwrap();
            assert_eq!(projected, [expected]);
            assert!(std::ptr::eq(projected.as_ptr(), &source[position]));
        }
        session.set_bus_expanded(0, true);
        assert_eq!(session.wave_rows().len(), 5);
        assert_eq!(session.trace.histories.len(), 1);
        assert_eq!(session.trace.signals.len(), 1);
    }
}

#[test]
fn expanded_aliases_move_with_their_bits_and_remain_independent() {
    let input = b"$scope module tb $end
$var wire 4 ! data [3:0] $end
$var wire 4 ! alias [0:3] $end
$var wire 1 \" clk $end
$upscope $end
$enddefinitions $end
b1010 !
0\"
#10
";
    let mut session = Session::new(oculi::vcd::load(&input[..]).unwrap());
    assert_eq!(session.selected, [0, 2]);
    session.set_selected(1, true);
    session.set_bus_expanded(0, true);
    session.set_bus_expanded(0, true);
    assert!(session.is_bus_expanded(0));
    assert!(!session.is_bus_expanded(1));
    let labels = |session: &Session| {
        session
            .wave_rows()
            .iter()
            .map(|row| row.label(&session.trace))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        labels(&session),
        [
            "tb.data[3:0]",
            "tb.data[3]",
            "tb.data[2]",
            "tb.data[1]",
            "tb.data[0]",
            "tb.clk",
            "tb.alias[0:3]",
        ]
    );
    session.move_selected(0, 2);
    assert_eq!(
        labels(&session),
        [
            "tb.clk",
            "tb.alias[0:3]",
            "tb.data[3:0]",
            "tb.data[3]",
            "tb.data[2]",
            "tb.data[1]",
            "tb.data[0]",
        ]
    );
    session.set_bus_expanded(1, true);
    assert_eq!(session.wave_rows().len(), 11);
    session.set_bus_expanded(0, false);
    assert!(session.is_bus_expanded(1));
    assert_eq!(session.wave_rows().len(), 7);
    session.set_selected(1, false);
    assert!(!session.is_bus_expanded(1));
    assert_eq!(session.wave_rows().len(), 2);
    session.set_selected(1, true);
    assert!(!session.is_bus_expanded(1));
    assert_eq!(session.trace.histories.len(), 2);
}

#[test]
fn scalar_missing_and_unselected_signals_cannot_expand() {
    let mut session = ranged_bus("[3:0]");
    session.set_bus_expanded(999, true);
    assert_eq!(session.wave_rows().len(), 1);
    session.set_selected(0, false);
    session.set_bus_expanded(0, true);
    assert!(!session.is_bus_expanded(0));
    assert!(session.wave_rows().is_empty());
    let trace = oculi::vcd::load(&b"$var wire 1 ! clk $end\n$enddefinitions $end\n"[..]).unwrap();
    let mut session = Session::new(trace);
    session.set_bus_expanded(0, true);
    assert!(!session.is_bus_expanded(0));
    assert_eq!(session.wave_rows().len(), 1);
}

#[test]
fn wide_buses_expand_without_integer_width_limits_or_new_histories() {
    let mut session = Session::new(
        oculi::vcd::load(
            &b"$var wire 132 ! wide [131:0] $end\n$enddefinitions $end\nb1 !\n#10\n"[..],
        )
        .unwrap(),
    );
    session.set_bus_expanded(0, true);
    let rows = session.wave_rows();
    assert_eq!(rows.len(), 133);
    assert_eq!(rows[1].label(&session.trace), "wide[131]");
    assert_eq!(rows[132].label(&session.trace), "wide[0]");
    let value = session.trace.histories[0].value_at(0).unwrap();
    assert_eq!(rows[1].project(value).unwrap(), [Zero]);
    assert_eq!(rows[132].project(value).unwrap(), [One]);
    assert_eq!(session.trace.histories.len(), 1);
    assert!(
        WaveRow {
            signal: 0,
            bit: Some(132)
        }
        .project(value)
        .is_none()
    );
}
