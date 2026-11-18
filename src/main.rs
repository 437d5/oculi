use std::{
    env,
    error::Error,
    fs::File,
    io::{self, BufReader, Write},
    path::PathBuf,
    process::ExitCode,
};

use oculi::trace::TimeUnit;

const USAGE: &str = "Usage: oculi <path.vcd>\n\nList digital signals, widths, and value transition counts in a completed VCD trace.\n\n  -h, --help    Show this help\n";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("oculi: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // OS strings preserve paths that are not valid UTF-8.
    let mut args = env::args_os().skip(1);
    let first = args.next().ok_or(USAGE)?;
    if args.next().is_some() {
        return Err(USAGE.into());
    }
    let mut output = io::BufWriter::new(io::stdout().lock());
    if first == "--help" || first == "-h" {
        write!(output, "{USAGE}")?;
        output.flush()?;
        return Ok(());
    }

    let path = PathBuf::from(first);
    let file = File::open(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let trace = oculi::vcd::load(BufReader::new(file))
        .map_err(|error| format!("{}: {error}", path.display()))?;

    // Load completely before printing, so invalid input cannot look successful.
    match trace.timescale {
        Some(scale) => {
            let unit = match scale.unit {
                TimeUnit::Seconds => "s",
                TimeUnit::Milliseconds => "ms",
                TimeUnit::Microseconds => "us",
                TimeUnit::Nanoseconds => "ns",
                TimeUnit::Picoseconds => "ps",
                TimeUnit::Femtoseconds => "fs",
            };
            writeln!(output, "Timescale: {} {unit}", scale.magnitude)?;
        }
        None => writeln!(output, "Timescale: unspecified (integer ticks)")?,
    }
    writeln!(output, "Signal\tBits\tTransitions")?;
    for signal in &trace.signals {
        let history = &trace.histories[signal.history];
        writeln!(
            output,
            "{}\t{}\t{}",
            signal.path.join("."),
            signal.width,
            history.transition_count()
        )?;
    }
    output.flush()?;
    Ok(())
}
