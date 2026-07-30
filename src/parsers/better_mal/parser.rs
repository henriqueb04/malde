use std::fmt::Display;
use std::{collections::HashMap, iter::Peekable, mem::discriminant};

use thiserror::Error;

use crate::architecture::signals::ControlSignals;
use crate::parsers::better_mal::{
    tokenizer::{Token, TokenType, Tokenizer, TokenizerError, TokenizerErrorType},
};
use crate::parsers::source_map::{SourceMap, Span};

pub struct MALParser<'a> {
    source_map: &'a SourceMap,
    lexer: Peekable<Tokenizer<'a>>,
    mappings: HashMap<String, usize>,
}

impl<'a> MALParser<'a> {
    pub fn new(source_map: &'a SourceMap) -> Self {
        MALParser {
            source_map,
            lexer: Tokenizer::new(source_map).peekable(),
            mappings: HashMap::new(),
        }
    }

    /*
    microprogram :: clock*
    clock :: (identifier ":")? statement* \n
    statement :: ( syscall | wr | rd | goto_expr | conditional | assignment ) ";"
    goto_expr :: then? goto identifier
    conditional :: if condition goto_expr
    condition :: "n" | "z"
    assignment :: destregister ":=" shifted | operation
    shifted :: shift "(" operation ")"
    operation :: inv | band | add | transparency
    transparency :: register
    add :: register "+" register
    band :: "band" "(" register "," register ")"
    inv :: "inv" "(" register ")"

    register = mbr or any register from register bank (ac, pc, etc.)
    destregister = alu, mar or any other register
    */
    pub fn parse(mut self) -> Vec<Microinstruction> {
        Vec::new()
    }

}

#[derive(Debug, PartialEq, Eq)]
struct ParsingError {
    pub span: Span,
    pub error_type: ParsingErrorType,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub struct MALParsingError<'a> {
    pub source_map: &'a SourceMap,
    pub span: Span,
    pub error_type: ParsingErrorType,
}

impl Display for MALParsingError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Erro ao ler linha {}, coluna {}:\n{}\n\n{}",
            self.span.line,
            self.span.col,
            self.source_map.highlight_in_line(&self.span),
            self.error_type
        )
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParsingErrorType {
    #[error(transparent)]
    TokenError(TokenizerErrorType),
}

impl From<TokenizerError> for ParsingError {
    fn from(value: TokenizerError) -> Self {
        ParsingError {
            span: value.span,
            error_type: ParsingErrorType::TokenError(value.error_type),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Microinstruction {
    pub content: String,
    pub mir: ControlSignals,
}

