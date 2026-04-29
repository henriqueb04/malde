use crate::parsers::lockable::{ConflictType, ControlSignalsLockable};
use crate::{architecture::datapath::get_registor_index, parsers::lockable::ValueAlreadySet};
use regex::{Captures, Regex};
use std::error::Error;
use std::{fmt::Display, sync::LazyLock};

pub fn parse_line<'a>(
    line: &'a str,
) -> Option<Result<(&'a str, ControlSignalsLockable<'a>), ParsingError<'a>>> {
    let mut mir = ControlSignalsLockable::new();
    let line_name_capture = LINE_NAME_R.captures(line)?;
    let line_name = line_name_capture.name("name")?.as_str();
    let line_content = line_name_capture.name("content")?.as_str();
    for expr in line_content.split(';') {
        if expr.trim().is_empty() {
            continue;
        }
        println!("{}", expr);
        if let Err(err) = parse_expr(expr, &mut mir) {
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
        mir.set_bool("rd", true)?;
        return Ok(mir);
    }
    if WR_R.captures(expr).is_some() {
        mir.set_bool("wr", true)?;
        return Ok(mir);
    }
    // Procura por goto
    if let Some(goto) = IF_GOTO_R.captures(expr) {
        if let Some(cond) = goto.name("cond").map(|c| c.as_str()) {
            match cond {
                "n" => {
                    mir.set_int("cond", 1)?;
                }
                "z" => {
                    mir.set_int("cond", 2)?;
                }
                _ => {
                    return Err(ParsingError::InvalidCondition(cond));
                }
            }
        }
        // Seta como "sempre desvie" (3) caso não tenha sido setada
        let _ = mir.set_int("cond", 3);
        if let Some(addr) = goto.name("addr") {
            mir.set_addr_symbol(addr.as_str())?;
        } else {
            return Err(ParsingError::InvalidExpression(expr));
        }
        return Ok(mir);
    }
    // Procura por expressões de atribuição (aaa := xxx)
    let Some(outter) = OUTTER_R.captures(expr) else {
        return Err(ParsingError::InvalidExpression(expr));
    };
    let Some(dest) = outter.name("dest") else {
        return Err(ParsingError::InvalidExpression(expr));
    };
    let Some(operation) = outter.name("operation") else {
        return Err(ParsingError::InvalidExpression(expr));
    };
    // Verificações
    let dest = dest.as_str();
    let dest_index = get_registor_index(dest);
    let dest_is_registor = dest_index.is_some();
    let dest_is_mbr = dest == "mbr";
    let dest_is_mar = dest == "mar";
    // Ativar ENC se tentar atribuir a um registrador
    if dest_is_registor {
        mir.set_bool("enc", true)?;
        mir.set_int("c", dest_index.unwrap())?;
    }
    // Ativar MBR se o atributo para este
    if dest_is_mbr {
        mir.set_bool("mbr", true)?;
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
            return Err(ParsingError::InvalidExpression(expr));
        };
        mir.set_int("sh", if sh.as_str() == "lshift" { 1 } else { 2 })?;
        let Some(shift_op) = shifter.name("operation") else {
            return Err(ParsingError::InvalidExpression(expr));
        };
        operation = shift_op.as_str();
    }
    // Depois de verificar o shifter
    let Some(op) = OPERATION_R.captures(operation) else {
        return Err(ParsingError::InvalidExpression(expr));
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
                    return Err(ParsingError::SignalAlreadyDefined(err));
                }
            }
            mir.set_bool("mar", true)?;
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
        mir.set_int("alu", 0)?;
    } else if op.name("band").is_some() {
        // Bitwise And
        // A
        check_reg_a(mir, &op, "aA")?;
        // B
        check_reg_b(mir, &op, "aB")?;
        // ALU
        mir.set_int("alu", 1)?;
    } else if op.name("inv").is_some() {
        // Not
        // A
        check_reg_a(mir, &op, "iA")?;
        // ALU
        mir.set_int("alu", 3)?;
    } else if transparency.is_some() {
        // Transparência
        // A
        check_reg_a(mir, &op, "transparency")?;
        mir.set_int("alu", 2)?;
    } else {
        return Err(ParsingError::InvalidExpression(expr));
    }
    Ok(mir)
}

fn check_reg_a<'a, 'b>(
    mir: &'b mut ControlSignalsLockable<'a>,
    op: &'b Captures<'a>,
    name: &'a str,
) -> Result<(), ParsingError<'a>>
    where 'a: 'b
{
    let Some(name) = op.name(name) else {
        return Err(ParsingError::InvalidExpression(op.get(0).map_or("", |m| m.as_str())));
    };
    let name = name.as_str();
    if name == "mbr" {
        mir.set_bool("amux", true)?;
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
    mir.set_bool("amux", false)?;
    mir.set_int("a", index)?;
    Ok(())
}

fn check_reg_b<'a, 'b>(
    mir: &'b mut ControlSignalsLockable<'a>,
    op: &'b Captures<'a>,
    name: &'a str,
) -> Result<(), ParsingError<'a>> {
    let Some(name) = op.name(name) else {
        return Err(ParsingError::InvalidExpression(op.get(0).map_or("", |m| m.as_str())));
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
    // Caso B esteja ocupado e A tenha o valor desejado em B, verifica se há possibilidade de
    // trocar A e B sem causar problemas, mas só se mar vai ser setado e !amux
    mir.set_int("b", index).or_else(|err| {
        if mir.get_bool("mar").unwrap_or(false) {
            mir.set_int_force("b", mir.get_int("a").unwrap_or(0));
            Ok(0)
        } else {
            Err(ParsingError::SignalAlreadyDefined(err))
        }
    })?;
    Ok(())
}

#[derive(Debug)]
pub enum ParsingError<'a> {
    SignalAlreadyDefined(ValueAlreadySet<'a>),
    InvalidExpression(&'a str),
    InvalidRegistor(&'a str),
    InvalidCondition(&'a str),
    ImpossiblePath(&'a str, &'a str),
    IlegalRegistor(&'a str, &'a str),
    UnrecognizedSymbol(&'a str),
    MicroinstructionOverflow,
}

impl<'a> Display for ParsingError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignalAlreadyDefined(err) => {
                let (before, after) = match err.conflict {
                    ConflictType::Bool { before, after } => (before.to_string(), after.to_string()),
                    ConflictType::Int { before, after } => (before.to_string(), after.to_string()),
                    ConflictType::Str { before, after } => (before.to_string(), after.to_string()),
                };
                write!(
                    f,
                    "Valores conflitantes para o registrador {} (antes: {}, depois: {})",
                    err.name, before, after,
                )
            }
            Self::InvalidExpression(expr) => write!(f, "Expressão inválida: \"{}\"", expr),
            Self::InvalidRegistor(name) => write!(f, "Registrador inválido: {}", name),
            Self::InvalidCondition(cond) => write!(f, "Condição de if inválida: {}", cond),
            Self::ImpossiblePath(dest, expr) => write!(
                f,
                "Não é possível levar a expressão \"{}\" para o registrador {}",
                expr, dest
            ),
            Self::IlegalRegistor(reg, expr) => write!(
                f,
                "Não é possível colocar o registrador {} no barramento indicado na expressão \"{}\"",
                reg, expr
            ),
            Self::UnrecognizedSymbol(symb) => write!(f, "Símbolo \"{}\" não foi reconhecido", symb),
            Self::MicroinstructionOverflow => write!(f, "Microinstruções demais para a memória"),
        }
    }
}

impl<'a> Error for ParsingError<'a> {}

impl<'a> From<ValueAlreadySet<'a>> for ParsingError<'a> {
    fn from(value: ValueAlreadySet<'a>) -> Self {
        Self::SignalAlreadyDefined(value)
    }
}

static LINE_NAME_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:(?<name>[^:]+):)?\s*(?<content>.*)\s*(?://)?").unwrap());
static OUTTER_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?<dest>[\d\w_-]+)\s*:=\s*(?<operation>.+)\s*").unwrap());
static OPERATION_R: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?<add>(?<sA>[^\s\+,]+)\s*\+\s*(?<sB>[^\s\+,]+))|(?<band>band\s*\(\s*(?<aA>[^\s\+,]+),\s*(?<aB>[^\s\+,]+)\s*\))|(?<inv>inv\s*\(\s*(?<iA>[^\s\+,]+)\s*\))|(?<transparency>[^\s\+,]+))$").unwrap()
});
static SHIFT_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<shift>lshift|rshift)\s*\((?<operation>.+)\)$").unwrap());
static RD_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*rd\s*$").unwrap());
static WR_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*wr\s*$").unwrap());
static IF_GOTO_R: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:if\s+(?<cond>z|n))?(?:\s*then)?\s*goto\s+(?<addr>[\d\w_-]+)\s*$").unwrap()
});
