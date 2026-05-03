use std::fmt::Display;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueConflictType<'a> {
    Bool{ before: bool, after: bool },
    Int{ before: u8, after: u8 },
    Str{ before: &'a str, after: &'a str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueAlreadySet<'a> {
    pub name: &'static str,
    pub conflict: ValueConflictType<'a>,
}

pub struct ParsingError<'a> {
    pub lineno: usize,
    pub content: &'a str,
    pub error_type: ParsingErrorType<'a>,
}

impl Display for ParsingError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Erro na linha {}: {}\n    \"{}\"",
            self.lineno + 1,
            self.error_type,
            self.content
        )
    }
}

#[derive(Debug)]
pub enum ParsingErrorType<'a> {
    SignalAlreadyDefined(ValueAlreadySet<'a>),
    InvalidExpression(&'a str),
    InvalidRegistor(&'a str),
    InvalidCondition(&'a str),
    ImpossiblePath(&'a str, &'a str),
    IlegalRegistor(&'a str, &'a str),
    UnrecognizedSymbol(&'a str),
    MicroinstructionOverflow,
}

impl<'a> Display for ParsingErrorType<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignalAlreadyDefined(err) => {
                let (before, after) = match err.conflict {
                    ValueConflictType::Bool { before, after } => (before.to_string(), after.to_string()),
                    ValueConflictType::Int { before, after } => (before.to_string(), after.to_string()),
                    ValueConflictType::Str { before, after } => (before.to_string(), after.to_string()),
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

impl<'a> Error for ParsingErrorType<'a> {}

impl<'a> From<ValueAlreadySet<'a>> for ParsingErrorType<'a> {
    fn from(value: ValueAlreadySet<'a>) -> Self {
        Self::SignalAlreadyDefined(value)
    }
}
