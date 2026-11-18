//! Convert VCD syntax into Oculi's format-independent trace model.

use std::{collections::HashMap, error::Error, fmt, io};

use ::vcd::{Command, IdCode, ScopeItem, SimulationCommand, VarType};

use crate::trace::{History, Logic, Signal, TimeUnit, Timescale, Trace};

/// A loading failure with context and an underlying I/O/parser error when present.
#[derive(Debug)]
pub struct LoadError {
    context: String,
    source: Option<io::Error>,
}

impl LoadError {
    fn invalid(context: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            source: None,
        }
    }

    fn parsing(context: String, source: io::Error) -> Self {
        Self {
            context,
            source: Some(source),
        }
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.context)?;
        if let Some(source) = &self.source {
            write!(f, ": {source}")?;
        }
        Ok(())
    }
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|error| error as &dyn Error)
    }
}

#[derive(Clone, Copy)]
struct Binding {
    history: usize,
    width: u32,
}

/// Read a complete digital VCD trace into memory.
///
/// Supports scalar and binary-vector declarations/changes, `$dumpvars`, and
/// `$dumpall`. Real, string, event, and dump-control data are explicitly rejected.
/// No default timescale is invented when the file omits it.
pub fn load(reader: impl io::BufRead) -> Result<Trace, LoadError> {
    // The parser requires whitespace after a token, including the final one.
    // Appending whitespace accepts a complete file without a final newline;
    // incomplete commands still fail when their required fields are absent.
    let terminated = io::Read::chain(reader, &b"\n"[..]);
    let mut parser = ::vcd::Parser::new(terminated);
    let header = read_header(&mut parser)?;
    let mut trace = Trace::default();
    if let Some((magnitude, unit)) = header.timescale {
        trace.timescale = Some(Timescale {
            magnitude,
            unit: time_unit(unit),
        });
    }

    let mut bindings = HashMap::new();
    declarations(&header.items, &mut Vec::new(), &mut bindings, &mut trace)?;
    let mut time = 0;
    let mut block = None;
    while let Some(command) = parser.next() {
        let line = parser.line();
        let command = command
            .map_err(|error| LoadError::parsing(format!("VCD data near line {line}"), error))?;
        let failure = |message: String| LoadError::invalid(format!("line {line}: {message}"));
        match command {
            Command::Timestamp(next) => {
                if block.is_some() {
                    return Err(failure("timestamp inside a dump snapshot".into()));
                }
                if next < time {
                    return Err(failure(format!("timestamp {next} precedes {time}")));
                }
                time = next;
            }
            Command::ChangeScalar(id, value) => {
                record(&mut trace, &bindings, id, time, vec![logic(value)], true)
                    .map_err(|error| failure(error.to_string()))?;
            }
            Command::ChangeVector(id, value) => {
                record(
                    &mut trace,
                    &bindings,
                    id,
                    time,
                    value.iter().map(logic).collect(),
                    false,
                )
                .map_err(|error| failure(error.to_string()))?;
            }
            Command::Comment(_) => {}
            Command::Begin(kind) => {
                if block.is_some() {
                    return Err(failure("nested dump command".into()));
                }
                if !matches!(
                    kind,
                    SimulationCommand::Dumpvars | SimulationCommand::Dumpall
                ) {
                    return Err(failure(format!("unsupported ${kind} command")));
                }
                block = Some(kind);
            }
            Command::End(kind) => {
                if block != Some(kind) {
                    return Err(failure("unmatched dump command end".into()));
                }
                block = None;
            }
            Command::ChangeReal(..) => return Err(failure("unsupported real value change".into())),
            Command::ChangeString(..) => {
                return Err(failure("unsupported string value change".into()));
            }
            other => return Err(failure(format!("unsupported VCD data command: {other:?}"))),
        }
    }
    if let Some(kind) = block {
        return Err(LoadError::invalid(format!(
            "unterminated ${kind} near line {}",
            parser.line()
        )));
    }
    trace.end_time = time;
    Ok(trace)
}

// Consume the library's parsed commands instead of parse_header(), which retains
// only the last timescale and would hide malformed or conflicting declarations.
fn read_header(parser: &mut ::vcd::Parser<impl io::BufRead>) -> Result<::vcd::Header, LoadError> {
    let mut header = ::vcd::Header::default();
    let mut scopes = Vec::<::vcd::Scope>::new();
    loop {
        let command = parser.next().ok_or_else(|| {
            LoadError::invalid("unterminated VCD header: expected $enddefinitions")
        })?;
        let line = parser.line();
        let command = command
            .map_err(|error| LoadError::parsing(format!("VCD header near line {line}"), error))?;
        let failure =
            |message: &str| LoadError::invalid(format!("VCD header near line {line}: {message}"));
        match command {
            Command::Timescale(magnitude, unit) if scopes.is_empty() => {
                if !matches!(magnitude, 1 | 10 | 100) {
                    return Err(failure("timescale magnitude must be 1, 10, or 100"));
                }
                if header.timescale.is_some() {
                    return Err(failure("duplicate timescale declaration"));
                }
                header.timescale = Some((magnitude, unit));
            }
            Command::Date(date) if scopes.is_empty() => header.date = Some(date),
            Command::Version(version) if scopes.is_empty() => header.version = Some(version),
            Command::ScopeDef(kind, name) => scopes.push(::vcd::Scope::new(kind, name)),
            Command::Upscope => {
                let scope = scopes.pop().ok_or_else(|| failure("unmatched $upscope"))?;
                push_header_item(&mut header, &mut scopes, ScopeItem::Scope(scope));
            }
            Command::VarDef(kind, width, code, reference, index) => {
                let var = ::vcd::Var::new(kind, width, code, reference, index);
                push_header_item(&mut header, &mut scopes, ScopeItem::Var(var));
            }
            Command::Comment(comment) => {
                push_header_item(&mut header, &mut scopes, ScopeItem::Comment(comment));
            }
            Command::Enddefinitions if scopes.is_empty() => return Ok(header),
            _ => return Err(failure("unexpected command or unclosed scope")),
        }
    }
}

fn push_header_item(header: &mut ::vcd::Header, scopes: &mut [::vcd::Scope], item: ScopeItem) {
    if let Some(scope) = scopes.last_mut() {
        scope.items.push(item);
    } else {
        header.items.push(item);
    }
}

fn declarations(
    items: &[ScopeItem],
    path: &mut Vec<String>,
    bindings: &mut HashMap<IdCode, Binding>,
    trace: &mut Trace,
) -> Result<(), LoadError> {
    for item in items {
        match item {
            ScopeItem::Scope(scope) => {
                path.push(scope.identifier.clone());
                declarations(&scope.items, path, bindings, trace)?;
                path.pop();
            }
            ScopeItem::Var(var) => {
                if !matches!(
                    var.var_type,
                    VarType::Integer
                        | VarType::Parameter
                        | VarType::Reg
                        | VarType::Supply0
                        | VarType::Supply1
                        | VarType::Time
                        | VarType::Tri
                        | VarType::TriAnd
                        | VarType::TriOr
                        | VarType::TriReg
                        | VarType::Tri0
                        | VarType::Tri1
                        | VarType::WAnd
                        | VarType::Wire
                        | VarType::WOr
                ) {
                    return Err(LoadError::invalid(format!(
                        "unsupported {} declaration: {}",
                        var.var_type, var.reference
                    )));
                }
                if var.size == 0 {
                    return Err(LoadError::invalid(format!(
                        "zero width for {}",
                        var.reference
                    )));
                }
                let bit_range = match var.index {
                    Some(::vcd::ReferenceIndex::Range(left, right)) => {
                        let range_width = (i64::from(left) - i64::from(right)).unsigned_abs() + 1;
                        if range_width != u64::from(var.size) {
                            return Err(LoadError::invalid(format!(
                                "range [{left}:{right}] for {} disagrees with width {}",
                                var.reference, var.size
                            )));
                        }
                        Some((left, right))
                    }
                    _ => None,
                };
                let binding = if let Some(existing) = bindings.get(&var.code) {
                    if existing.width != var.size {
                        return Err(LoadError::invalid(format!(
                            "alias {} has width {}, expected {}",
                            var.reference, var.size, existing.width
                        )));
                    }
                    *existing
                } else {
                    let binding = Binding {
                        history: trace.histories.len(),
                        width: var.size,
                    };
                    trace.histories.push(History::default());
                    bindings.insert(var.code, binding);
                    binding
                };
                let mut signal_path = path.clone();
                let reference = match var.index {
                    Some(index) => format!("{}{index}", var.reference),
                    None => var.reference.clone(),
                };
                signal_path.push(reference);
                trace.signals.push(Signal {
                    path: signal_path,
                    width: var.size,
                    bit_range,
                    history: binding.history,
                });
            }
            ScopeItem::Comment(_) => {}
            other => {
                return Err(LoadError::invalid(format!(
                    "unsupported header item: {other:?}"
                )));
            }
        }
    }
    Ok(())
}

fn record(
    trace: &mut Trace,
    bindings: &HashMap<IdCode, Binding>,
    id: IdCode,
    time: u64,
    value: Vec<Logic>,
    scalar: bool,
) -> Result<(), LoadError> {
    let binding = bindings
        .get(&id)
        .ok_or_else(|| LoadError::invalid(format!("undeclared identifier {id}")))?;
    let width = binding.width as usize;
    if scalar && width != 1 {
        return Err(LoadError::invalid(format!(
            "scalar value for {width}-bit identifier {id}"
        )));
    }
    if value.is_empty() || value.len() > width {
        return Err(LoadError::invalid(format!(
            "value width {} exceeds declared width {width} for {id}",
            value.len()
        )));
    }
    // VCD abbreviates leading zero bits, or a run of X/Z bits. A leading one
    // still extends with zero, not with one (this is not signed extension).
    let fill = match value[0] {
        Logic::X => Logic::X,
        Logic::Z => Logic::Z,
        _ => Logic::Zero,
    };
    let mut normalized = vec![fill; width - value.len()];
    normalized.extend(value);
    trace.histories[binding.history].record(time, normalized);
    Ok(())
}

fn logic(value: ::vcd::Value) -> Logic {
    match value {
        ::vcd::Value::V0 => Logic::Zero,
        ::vcd::Value::V1 => Logic::One,
        ::vcd::Value::X => Logic::X,
        ::vcd::Value::Z => Logic::Z,
    }
}

fn time_unit(unit: ::vcd::TimescaleUnit) -> TimeUnit {
    match unit {
        ::vcd::TimescaleUnit::S => TimeUnit::Seconds,
        ::vcd::TimescaleUnit::MS => TimeUnit::Milliseconds,
        ::vcd::TimescaleUnit::US => TimeUnit::Microseconds,
        ::vcd::TimescaleUnit::NS => TimeUnit::Nanoseconds,
        ::vcd::TimescaleUnit::PS => TimeUnit::Picoseconds,
        ::vcd::TimescaleUnit::FS => TimeUnit::Femtoseconds,
    }
}
