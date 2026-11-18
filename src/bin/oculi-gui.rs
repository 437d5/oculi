use oculi::gui::{Startup, USAGE};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Startup::parse(std::env::args_os().skip(1))? {
        Startup::Help => println!("{USAGE}"),
        Startup::Open(path) => oculi::gui::run(path)?,
    }
    Ok(())
}
