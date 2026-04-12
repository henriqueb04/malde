use crate::architecture::datapath::{get_registor_index, REGISTOR_NAMES};
use crate::parsers::lockable::ControlSignalsLockable;
use regex::{Captures, Regex};
use std::sync::LazyLock;

struct MALParser {}

pub enum ParsingError<'a> {
    SignalAlreadyDefined(String),
    InvalidStatement,
    InvalidRegistor(&'a str),
    ImpossiblePath(&'a str, &'a str),
}

static LINE_NAME_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*([^:]+):\s*").unwrap());
static OUTTER_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?<dest>\S+)\s*:=\s*(?<operation>.+)\s*").unwrap());
static OPERATION_R: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?<add>(?<sA>[^\s\+,]+)\s*\+\s*(?<sB>[^\s\+,]+))|(?<band>band\(\s*(?<aA>[^\s\+,]+),\s*(?<aB>[^\s\+,]+)\s*\))|(?<inv>inv\(\s*(?<iA>[^\s\+,]+)\s*\))|(?<transparency>[^\s\+,]+))$").unwrap()
});
static SHIFT_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<shift>lshift|rshift)\((?<operation>.+)\)$").unwrap());

fn get_matched_registor_index<'a, 'b>(
    op: &'b Captures<'a>,
    name: &'a str,
) -> Result<u8, ParsingError<'a>> {
    let Some(name) = op.name(name) else {
        return Err(ParsingError::InvalidStatement);
    };
    let Some(index) = get_registor_index(name.as_str()) else {
        return Err(ParsingError::InvalidRegistor(name.as_str()));
    };
    Ok(index)
}

fn parse_expr<'a, 'b>(
    line: &'a str,
    mir: &'b mut ControlSignalsLockable,
) -> Result<&'b ControlSignalsLockable, ParsingError<'a>> {
    let Some(outter) = OUTTER_R.captures(line) else {
        return Err(ParsingError::InvalidStatement);
    };
    let Some(dest) = outter.name("dest") else {
        return Err(ParsingError::InvalidStatement);
    };
    let Some(operation) = outter.name("operation") else {
        return Err(ParsingError::InvalidStatement);
    };
    let dest = dest.as_str();
    if !(REGISTOR_NAMES.contains(&dest) || dest == "mar" || dest == "mbr") {
        return Err(ParsingError::InvalidRegistor(dest));
    }
    let mut operation = operation.as_str();
    let dest_is_mar = dest == "mar";
    let shifter = SHIFT_R.captures(operation);
    if let Some(shifter) = shifter {
        if dest_is_mar {
            return Err(ParsingError::ImpossiblePath(
                dest,
                shifter.get(0).unwrap().as_str(),
            ));
        }
        let Some(sh) = shifter.name("shift") else {
            return Err(ParsingError::InvalidStatement);
        };
        if let Err(err) = mir.set_sh(if sh.as_str() == "lshift" { 1 } else { 2 }) {
            return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
        }
        let Some(shift_op) = shifter.name("operation") else {
            return Err(ParsingError::InvalidStatement);
        };
        operation = shift_op.as_str();
    }
    let Some(op) = OPERATION_R.captures(operation) else {
        return Err(ParsingError::InvalidStatement);
    };
    if op.name("add").is_some() {
        // Add
        // A
        match get_matched_registor_index(&op, "sA") {
            Ok(index) => if let Err(err) = mir.set_a(index) {
                return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
            },
            Err(err) => return Err(err),
        }
        // B
        match get_matched_registor_index(&op, "sB") {
            Ok(index) => if let Err(err) = mir.set_b(index) {
                return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
            },
            Err(err) => return Err(err),
        }
        if let Err(err) = mir.set_alu(0) {
            return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
        }
    } else if op.name("band").is_some() {
        // Bitwise And
        // A
        match get_matched_registor_index(&op, "aA") {
            Ok(index) => if let Err(err) = mir.set_a(index) {
                return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
            },
            Err(err) => return Err(err),
        }
        // B
        match get_matched_registor_index(&op, "aB") {
            Ok(index) => if let Err(err) = mir.set_b(index) {
                return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
            },
            Err(err) => return Err(err),
        }
        if let Err(err) = mir.set_alu(1) {
            return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
        }
    } else if op.name("inv").is_some() {
        // Not
        // A
        match get_matched_registor_index(&op, "iA") {
            Ok(index) => if let Err(err) = mir.set_a(index) {
                return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
            },
            Err(err) => return Err(err),
        }
        if let Err(err) = mir.set_alu(3) {
            return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
        }
    } else if op.name("transparecy").is_some() {
        // Transparency
        // A
        match get_matched_registor_index(&op, "transparency") {
            Ok(index) => if let Err(err) = mir.set_a(index) {
                return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
            },
            Err(err) => return Err(err),
        }
        if let Err(err) = mir.set_alu(2) {
            return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
        }
    } else {
        return Err(ParsingError::InvalidStatement);
    }
    Ok(mir)
}

fn parse_line<'a>(
    line: &'a str,
) -> Option<Result<(&'a str, ControlSignalsLockable), ParsingError<'a>>> {
    let mut mir = ControlSignalsLockable::default();
    let line_name_capture = LINE_NAME_R.captures(line)?;
    let line_name = line_name_capture.get(1)?.as_str();
    let line_content = &line[line_name_capture.get(0)?.end()..];
    for expr in line_content.split(';') {
        if let Err(err) = parse_expr(expr, &mut mir) {
            return Some(Err(err));
        }
    }
    Some(Ok((line_name, mir)))
}

impl MALParser {
    fn new() -> Self {
        MALParser {}
    }
}
