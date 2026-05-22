use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum ParsingErrorType<'a> {
    InvalidLine,
    InvalidNumber(&'a str),
    DuplicatedIdentifier(&'a str),
    UndefinedIdentifier(&'a str),
    InstructionTooBig(String, String, usize),
    NumberTooBig(isize, usize),
    NumberTooSmall(isize, usize),
    InvalidInstruction(String),
    UnsupportedDirective(&'a str),
    InvalidString(&'a str),
    InvalidChar(&'a str),
    InstructionOverflow(usize),
    DataOverflow(usize),

}

impl Display for ParsingErrorType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLine => write!(f, "Linha inválida"),
            Self::InvalidNumber(num) => write!(f, "Número inválido \"{}\"", num),
            Self::DuplicatedIdentifier(iden) => {
                write!(f, "Identificador \"{}\" declarado mais de uma vez", iden)
            }
            Self::UndefinedIdentifier(iden) => {
                write!(f, "Identificador \"{}\" não encontrador", iden)
            }
            Self::InstructionTooBig(ins, bin, len) => write!(
                f,
                "Instrução \"{}\" ({}) excede o tamanho de 16 bits, tem tamanho {}",
                ins, bin, len
            ),
            Self::NumberTooBig(n, len) => {
                write!(f, "Número {} grande demais para tamanho {}", n, len)
            }
            Self::NumberTooSmall(n, len) => {
                write!(f, "Número {} pequeno demais para tamanho {}", n, len)
            }
            Self::InvalidInstruction(ins) => write!(f, "Instrução inválida \"{}\"", ins),
            Self::UnsupportedDirective(dir) => write!(f, "Diretiva \"{}\" não suportada", dir),
            // Self::InvalidDataDefinition(_reason, content) => write!(f, "Declaração data inválida \"{}\"", content),
            Self::InvalidString(content) => write!(f, "String inválida '{}'", content),
            Self::InvalidChar(content) => write!(f, "Caractere inválido '{}'", content),
            Self::InstructionOverflow(size) => write!(f, "Quantidade {} de instruções maior que o tamanho da seção de instruções", size),
            Self::DataOverflow(size) => write!(f, "Quantidade {} de dados maior que o tamanho da seção de dados", size),
        }
    }
}

#[derive(Debug)]
pub struct ParsingError<'a> {
    pub lineno: usize,
    pub content: &'a str,
    pub error_type: ParsingErrorType<'a>,
}

impl Display for ParsingError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Erro no macroprograma, linha {}: {}\n{}",
            self.lineno, self.content, self.error_type
        )
    }
}

impl Error for ParsingError<'_> {}
