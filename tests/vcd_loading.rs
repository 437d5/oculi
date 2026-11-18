use oculi::trace::{Logic::*, TimeUnit, Timescale};
use oculi::vcd::load;

#[test]
fn retains_quiet_time_and_full_width_timestamps() {
    for (body, end) in [
        ("#5\n0!\n#100\n", 100),
        ("0!\n", 0),
        ("#18446744073709551615\n", u64::MAX),
    ] {
        let result = load(trace(body, 1).as_bytes()).unwrap();
        assert_eq!(result.end_time, end);
    }
}

fn trace(body: &str, width: u32) -> String {
    format!(
        "$timescale 10 ps $end\n$scope module tb $end\n$var wire {width} ! data $end\n$upscope $end\n$enddefinitions $end\n{body}"
    )
}

#[test]
fn loads_hierarchy_timescale_four_states_and_aliases() {
    let input = b"$timescale 1 ns $end
$scope module tb $end
$var wire 1 ! clk $end
$scope module dut $end
$var wire 1 ! clock $end
$var reg 4 \" count [3:0] $end
$upscope $end
$upscope $end
$enddefinitions $end
$dumpvars
0!
bx \"
$end
#5
1!
b1 \"
#6
1!
b0001 \"
#10
z!
bz \"
";
    let result = load(&input[..]).unwrap();
    assert_eq!(
        result.timescale,
        Some(Timescale {
            magnitude: 1,
            unit: TimeUnit::Nanoseconds
        })
    );
    assert_eq!(result.signals[0].path, ["tb", "clk"]);
    assert_eq!(result.signals[1].path, ["tb", "dut", "clock"]);
    assert_eq!(result.signals[2].path, ["tb", "dut", "count[3:0]"]);
    assert_eq!(result.histories.len(), 2);
    assert_eq!(result.signals[0].history, result.signals[1].history);
    let clock = &result.histories[result.signals[0].history];
    assert_eq!(clock.transition_count(), 2);
    assert_eq!(clock.changes()[2].value, [Z]);
    assert_eq!(clock.changes()[1].time, 5);
    let count = &result.histories[result.signals[2].history];
    assert_eq!(count.transition_count(), 2);
    assert_eq!(count.changes()[0].value, [X, X, X, X]);
    assert_eq!(count.changes()[1].value, [Zero, Zero, Zero, One]);
    assert_eq!(count.changes()[2].value, [Z, Z, Z, Z]);
}

#[test]
fn rejects_bus_ranges_that_disagree_with_the_recorded_width() {
    for range in ["[7:0]", "[0:7]", "[-1:-8]", "[0:0]"] {
        let input = format!("$var wire 4 ! data {range} $end\n$enddefinitions $end\n");
        let error = load(input.as_bytes()).unwrap_err().to_string();
        assert!(
            error.contains("range") && error.contains("width"),
            "{error}"
        );
    }
}

#[test]
fn normalizes_short_vectors_before_comparing_values() {
    let input = trace("b1 !\n#1\nb0001 !\n#2\nbx !\n#3\nbxxxx !\n#4\nbz !\n", 4);
    let result = load(input.as_bytes()).unwrap();
    let history = &result.histories[0];
    assert_eq!(history.transition_count(), 2);
    assert_eq!(history.changes()[0].value, [Zero, Zero, Zero, One]);
    assert_eq!(history.changes()[1].value, [X, X, X, X]);
    assert_eq!(history.changes()[2].value, [Z, Z, Z, Z]);
}

#[test]
fn preserves_equal_time_changes_and_missing_timescale() {
    let input = trace("#2\n0!\n#2\n1!\n#2\nx!\n", 1).replace("$timescale 10 ps $end\n", "");
    let result = load(input.as_bytes()).unwrap();
    assert_eq!(result.timescale, None);
    let changes = result.histories[0].changes();
    assert_eq!(
        changes.iter().map(|c| c.time).collect::<Vec<_>>(),
        [2, 2, 2]
    );
    assert_eq!(
        changes.iter().map(|c| c.value[0]).collect::<Vec<_>>(),
        [Zero, One, X]
    );
}

#[test]
fn rejects_invalid_or_unsupported_changes_with_context() {
    for (body, width, expected) in [
        ("b10101 !\n", 4, "width"),
        ("0!\n", 4, "scalar"),
        ("0?\n", 1, "undeclared"),
        ("#5\n0!\n#4\n1!\n", 1, "timestamp"),
        ("r1.5 !\n", 1, "real"),
        ("shello !\n", 1, "string"),
        ("$dumpoff\n$end\n", 1, "dumpoff"),
        ("$dumpvars\n0!\n", 1, "unterminated"),
        ("$dumpvars\n$dumpall\n$end\n", 1, "nested"),
        ("b102 !\n", 4, "line"),
    ] {
        let input = trace(body, width);
        let error = load(input.as_bytes()).unwrap_err().to_string();
        assert!(error.contains(expected), "{body:?}: {error}");
    }
}

#[test]
fn rejects_invalid_headers_and_unsupported_declarations() {
    for (input, expected) in [
        ("$scope module tb $end\n", "header"),
        ("$timescale 0 ns $end\n$enddefinitions $end\n", "timescale"),
        ("$var wire 0 ! bad $end\n$enddefinitions $end\n", "width"),
        (
            "$var wire 1 ! a $end\n$var wire 2 ! b $end\n$enddefinitions $end\n",
            "alias",
        ),
        (
            "$var real 64 ! voltage $end\n$enddefinitions $end\n",
            "real",
        ),
        (
            "$var string 1 ! state $end\n$enddefinitions $end\n",
            "string",
        ),
        ("$var event 1 ! tick $end\n$enddefinitions $end\n", "event"),
    ] {
        let error = load(input.as_bytes()).unwrap_err().to_string();
        assert!(error.contains(expected), "{input:?}: {error}");
    }
}

#[test]
fn retains_underlying_io_error() {
    struct Broken;
    impl std::io::Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken reader"))
        }
    }
    let error = load(std::io::BufReader::new(Broken)).unwrap_err();
    assert!(std::error::Error::source(&error).is_some());
    assert!(error.to_string().contains("broken reader"));
}

#[test]
fn rejects_timestamps_inside_dump_snapshots() {
    for command in ["dumpvars", "dumpall"] {
        let input = trace(&format!("${command}\n0!\n#5\n1!\n$end\n"), 1);
        let error = load(input.as_bytes()).unwrap_err().to_string();
        assert!(
            error.contains("timestamp") && error.contains("snapshot"),
            "{error}"
        );
    }
}

#[test]
fn validates_every_timescale_instead_of_only_the_last() {
    for scales in [
        "$timescale 0 ns $end\n$timescale 1 ps $end\n",
        "$timescale 1 ns $end\n$timescale 1 ps $end\n",
    ] {
        let input = format!("{scales}$enddefinitions $end\n");
        let error = load(input.as_bytes()).unwrap_err().to_string();
        assert!(error.contains("timescale"), "{error}");
    }
}

#[test]
fn accepts_a_complete_final_token_without_a_trailing_newline() {
    let input = trace("#5\n0!", 1);
    let result = load(input.as_bytes()).unwrap();
    assert_eq!(result.histories[0].changes()[0].time, 5);
    assert_eq!(result.histories[0].changes()[0].value, [Zero]);
    for incomplete in ["#", "b1", "0"] {
        assert!(
            load(trace(incomplete, 1).as_bytes()).is_err(),
            "{incomplete}"
        );
    }
}
