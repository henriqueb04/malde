use crate::architecture::datapath::get_registor_index;
use crate::parsers::lockable::ControlSignalsLockable;
use regex::{Captures, Regex};
use std::sync::LazyLock;

// Macros para não ter que ficar lidando com um possível erro toda vez.
// Se der erro, elas jogam pra cima e vai na fé
macro_rules! set_int {
    ($mir:expr, $name:expr, $val:expr) => {
        if let Err(err) = $mir.set_int($name, $val) {
            return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
        }
    };
}
macro_rules! set_bool {
    ($mir:expr, $name:expr, $val:expr) => {
        if let Err(err) = $mir.set_bool($name, $val) {
            return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
        }
    };
}

pub fn parse_line<'a>(
    line: &'a str,
) -> Option<Result<(&'a str, ControlSignalsLockable<'a>), ParsingError<'a>>> {
    let mut mir = ControlSignalsLockable::new();
    let line_name_capture = LINE_NAME_R.captures(line)?;
    let line_name = line_name_capture.get(1)?.as_str();
    let line_content = &line[line_name_capture.get(0)?.end()..];
    for expr in line_content.split(';') {
        if expr.trim().is_empty() {
            continue;
        }
        if let Err(err) = parse_expr(expr, &mut mir) {
            println!("{}", expr);
            return Some(Err(err));
        }
    }
    // Valores padrão, são ignorados se já estiverem presentes
    let _ = mir.set_bool("amux", false);
    let _ = mir.set_bool("enc", false);
    let _ = mir.set_bool("mar", false);
    let _ = mir.set_bool("mbr", false);
    let _ = mir.set_bool("rd", false);
    let _ = mir.set_bool("wr", false);
    if mir.get_bool("enc").unwrap_or(false) || mir.get_bool("mbr").unwrap_or(false) {
        let _ = mir.set_int("sh", 0);
    }
    let _ = mir.set_int("cond", 0);
    Some(Ok((line_name, mir)))
}

fn parse_expr<'a, 'b>(
    expr: &'a str,
    mir: &'b mut ControlSignalsLockable<'a>,
) -> Result<&'b ControlSignalsLockable<'a>, ParsingError<'a>> {
    // Procura rd ou wr
    if RD_R.captures(expr).is_some() {
        set_bool!(mir, "rd", true);
        return Ok(mir);
    }
    if WR_R.captures(expr).is_some() {
        set_bool!(mir, "wr", true);
        return Ok(mir);
    }
    // Procura por goto
    if let Some(goto) = IF_GOTO_R.captures(expr) {
        if let Some(cond) = goto.name("cond") {
            match cond.as_str() {
                "n" => {
                    if let Err(err) = mir.set_int("cond", 1) {
                        return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
                    }
                }
                "z" => {
                    if let Err(err) = mir.set_int("cond", 2) {
                        return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
                    }
                }
                _ => {
                    if let Err(err) = mir.set_int("cond", 3) {
                        return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
                    }
                }
            }
        }
        if let Some(addr) = goto.name("addr") {
            if let Err(err) = mir.set_addr_symbol(addr.as_str()) {
                return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
            }
        } else {
            return Err(ParsingError::InvalidStatement);
        }
        return Ok(mir);
    }
    // Procura por expressões de atribuição (aaa := xxx)
    let Some(outter) = OUTTER_R.captures(expr) else {
        return Err(ParsingError::InvalidStatement);
    };
    let Some(dest) = outter.name("dest") else {
        return Err(ParsingError::InvalidStatement);
    };
    let Some(operation) = outter.name("operation") else {
        return Err(ParsingError::InvalidStatement);
    };
    // Verificações
    let dest = dest.as_str();
    let dest_index = get_registor_index(dest);
    let dest_is_registor = dest_index.is_some();
    let dest_is_mbr = dest == "mbr";
    let dest_is_mar = dest == "mar";
    // Ativar ENC se tentar atribuir a um registrador
    if dest_is_registor {
        set_bool!(mir, "enc", true);
        if let Err(err) = mir.set_int("c", dest_index.unwrap()) {
            return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
        }
    }
    // Ativar MBR se o atributo para este
    if dest_is_mbr {
        set_bool!(mir, "mbr", true);
    }
    // Começo da operação
    let mut operation = operation.as_str();
    // Verifica se é preciso deslocar
    let shifter = SHIFT_R.captures(operation);
    if let Some(shifter) = shifter {
        // Não dá pra chegar em MAR depois de passar pelo deslocador
        if dest_is_mar {
            return Err(ParsingError::ImpossiblePath(
                dest,
                shifter.get(0).unwrap().as_str(),
            ));
        }
        // Só pra garantir que o regex não falhou
        let Some(sh) = shifter.name("shift") else {
            return Err(ParsingError::InvalidStatement);
        };
        if let Err(err) = mir.set_int("sh", if sh.as_str() == "lshift" { 1 } else { 2 }) {
            return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
        }
        let Some(shift_op) = shifter.name("operation") else {
            return Err(ParsingError::InvalidStatement);
        };
        operation = shift_op.as_str();
    }
    // Depois de verificar o shifter
    let Some(op) = OPERATION_R.captures(operation) else {
        return Err(ParsingError::InvalidStatement);
    };
    // É impossível realizar qualquer operação quando atribuindo valor ao MAR, portanto,
    // usamos a "operação" de tranparência
    let transparency = op.name("transparency");
    if dest_is_mar {
        if let Some(name) = transparency {
            let name = name.as_str();
            let Some(index) = get_registor_index(name) else {
                if name == "mar" || name == "mbr" {
                    return Err(ParsingError::ImpossiblePath(dest, name));
                }
                return Err(ParsingError::InvalidRegistor(name));
            };
            // Caso B esteja ocupado e A tenha o valor desejado em B, verifica se há possibilidade de
            // trocar A e B sem causar problemas, mas só se isso não já tiver sido feito e !amux
            if let Err(err) = mir.set_int("b", index) {
                if !mir.get_bool("amux").unwrap_or(false)
                    && !mir.get_bool("mar").unwrap_or(false)
                    && mir.get_int("a").eq(&Some(index))
                {
                    mir.swap_a_b();
                } else {
                    return Err(ParsingError::SignalAlreadyDefined(err.to_string()));
                }
            }
            set_bool!(mir, "mar", true);
        } else {
            return Err(ParsingError::ImpossiblePath(
                dest,
                op.get(0).unwrap().as_str(),
            ));
        }
        return Ok(mir);
    }
    if op.name("add").is_some() {
        // Soma
        // A
        check_reg_a(mir, &op, "sA")?;
        // B
        check_reg_b(mir, &op, "sB")?;
        // ALU
        set_int!(mir, "alu", 0);
    } else if op.name("band").is_some() {
        // Bitwise And
        // A
        check_reg_a(mir, &op, "aA")?;
        // B
        check_reg_b(mir, &op, "aB")?;
        // ALU
        set_int!(mir, "alu", 1);
    } else if op.name("inv").is_some() {
        // Not
        // A
        check_reg_a(mir, &op, "iA")?;
        // ALU
        set_int!(mir, "alu", 3);
    } else if transparency.is_some() {
        // Transparência
        // A
        check_reg_a(mir, &op, "transparency")?;
        set_int!(mir, "alu", 2);
    } else {
        return Err(ParsingError::InvalidStatement);
    }
    Ok(mir)
}

fn check_reg_a<'a, 'b>(
    mir: &'b mut ControlSignalsLockable<'a>,
    op: &'b Captures<'a>,
    name: &'a str,
) -> Result<(), ParsingError<'a>> {
    let Some(name) = op.name(name) else {
        return Err(ParsingError::InvalidStatement);
    };
    let name = name.as_str();
    if name == "mbr" {
        set_bool!(mir, "amux", true);
        return Ok(());
    } else if name == "mar" {
        return Err(ParsingError::IlegalRegistor(
            name,
            op.get(0).unwrap().as_str(),
        ));
    }
    let Some(index) = get_registor_index(name) else {
        return Err(ParsingError::InvalidRegistor(name));
    };
    set_bool!(mir, "amux", false);
    set_int!(mir, "a", index);
    Ok(())
}

fn check_reg_b<'a, 'b>(
    mir: &'b mut ControlSignalsLockable<'a>,
    op: &'b Captures<'a>,
    name: &'a str,
) -> Result<(), ParsingError<'a>> {
    let Some(name) = op.name(name) else {
        return Err(ParsingError::InvalidStatement);
    };
    let name = name.as_str();
    if name == "mbr" || name == "mar" {
        return Err(ParsingError::IlegalRegistor(
            name,
            op.get(0).unwrap().as_str(),
        ));
    }
    let Some(index) = get_registor_index(name) else {
        return Err(ParsingError::InvalidRegistor(name));
    };
    set_int!(mir, "b", index);
    Ok(())
}

#[derive(Debug)]
pub enum ParsingError<'a> {
    SignalAlreadyDefined(String),
    InvalidStatement,
    InvalidRegistor(&'a str),
    InvalidCondition(&'a str),
    ImpossiblePath(&'a str, &'a str),
    IlegalRegistor(&'a str, &'a str),
    UnrecognizedSymbol(&'a str),
    MicroinstructionOverflow,
}

static LINE_NAME_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:(?<name>[^:]+):)?\s*(<content>.*)(?://)?").unwrap());
static OUTTER_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?<dest>[\d\w_-]+)\s*:=\s*(?<operation>.+)\s*").unwrap());
static OPERATION_R: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?<add>(?<sA>[^\s\+,]+)\s*\+\s*(?<sB>[^\s\+,]+))|(?<band>band\(\s*(?<aA>[^\s\+,]+),\s*(?<aB>[^\s\+,]+)\s*\))|(?<inv>inv\(\s*(?<iA>[^\s\+,]+)\s*\))|(?<transparency>[^\s\+,]+))$").unwrap()
});
static SHIFT_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<shift>lshift|rshift)\((?<operation>.+)\)$").unwrap());
static RD_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*rd\s*$").unwrap());
static WR_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*wr\s*$").unwrap());
static IF_GOTO_R: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:if\s+(?<cond>z|n)\s*then)?\s*goto\s+(?<addr>[\d\w_-]+)\s*$").unwrap()
});
