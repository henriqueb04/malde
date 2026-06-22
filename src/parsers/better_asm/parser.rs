use std::{collections::HashMap, iter::Peekable, mem::discriminant};

use thiserror::Error;

use crate::parsers::better_asm::tokenizer::{
    SourceMap, Span, Token, TokenType, Tokenizer, TokenizerError, TokenizerErrorType,
};

pub struct ASMParser<'a> {
    source_map: SourceMap<'a>,
    lexer: Peekable<Tokenizer<'a>>,
    data: Vec<u16>,
    ins: Vec<u16>,
    data_mappings: HashMap<&'a str, usize>,
    ins_mappings: HashMap<&'a str, usize>,
}

impl<'a> ASMParser<'a> {
    pub fn new(source_map: SourceMap<'a>) -> Self {
        ASMParser {
            source_map: source_map.clone(),
            lexer: Tokenizer::new(&source_map).peekable(),
            data: Vec::new(),
            ins: Vec::new(),
            data_mappings: HashMap::new(),
            ins_mappings: HashMap::new(),
        }
    }

    pub fn parse(mut self) -> Result<Vec<u16>, ParsingError> {
        while self.lexer.peek().cloned().transpose()?.is_some() {
            self.read_section()?;
        }
        Ok(self.data)
    }

    fn read_section(&mut self) -> Result<(), ParsingError> {
        let t = self.expect(TokenType::Directive)?;
        let sec = self.source_map.get_span(&t.span);
        let _ = match sec {
            ".data" => self.read_data(),
            ".text" => self.read_text(),
            _ => {
                return Err(ParsingError {
                    span: t.span,
                    error_type: ParsingErrorType::UnrecognizedSession(sec.to_owned()),
                });
            }
        };
        Ok(())
    }

    fn read_data(&mut self) -> Result<(), ParsingError> {
        let t = self.expect(TokenType::Identifier)?;
        self.expect(TokenType::Colon)?;
        let dir = self.expect(TokenType::Directive)?;
        let data_start = self.data.len();
        let source = self.source_map.get_span(&dir.span);
        match source {
            ".ascii" => self.data_add_string()?,
            ".asciz" | ".asciiz" => {
                self.data_add_string()?;
                self.data.push(0);
            }
            ".word" => {
                self.data_add_number(i16::MIN as isize, u16::MAX as isize)?;
            }
            ".byte" => {
                self.data_add_number(i8::MIN as isize, u8::MAX as isize)?;
            }
            ".space" => {
                let t = self.expect(TokenType::Int(0))?;
                if let TokenType::Int(n) = t.token_type {
                    if n < 0 {
                        return Err(ParsingError {
                            span: t.span,
                            error_type: ParsingErrorType::NumberTooLow(n, 0),
                        });
                    }
                    self.data
                        .resize(self.data.len() + (n as usize).div_ceil(2), 0);
                }
            }
            _ => {
                return Err(ParsingError {
                    span: dir.span,
                    error_type: ParsingErrorType::UnsupportedDirective,
                });
            }
        }
        self.data_mappings
            .insert(self.source_map.get(t), data_start);
        while let Some(t) = self.lexer.peek().cloned().transpose()?
            && t.token_type == TokenType::Semicolon
        {
            self.lexer.next();
        }
        if let Some(t) = self.lexer.peek().cloned().transpose()?
            && t.token_type == TokenType::Identifier
        {
            self.read_data()?;
        }
        Ok(())
    }

    // TODO
    fn read_text(&mut self) -> Result<(), ParsingError> {
        Ok(())
    }

    fn expect(&mut self, typ: TokenType) -> Result<Token, ParsingError> {
        let t = self.lexer.next().ok_or(ParsingError {
            span: self.source_map.end(),
            error_type: ParsingErrorType::UnexpectedEnd,
        })??;
        if discriminant(&t.token_type) != discriminant(&typ) {
            return Err(ParsingError {
                span: t.span.clone(),
                error_type: ParsingErrorType::UnexpectedToken(t, typ),
            });
        }
        Ok(t)
    }

    fn data_add_string(&mut self) -> Result<(), ParsingError> {
        let s = self.expect(TokenType::String(String::new()))?;
        if let TokenType::String(seq) = s.token_type {
            for (_, c) in seq.char_indices() {
                self.data.push(c as u16);
            }
        }
        Ok(())
    }
    fn data_add_number(&mut self, min: isize, max: isize) -> Result<(), ParsingError> {
        let t = self.expect(TokenType::Int(0))?;
        if let TokenType::Int(n) = t.token_type {
            if n < min {
                return Err(ParsingError {
                    span: t.span,
                    error_type: ParsingErrorType::NumberTooLow(n, min),
                });
            } else if n > max {
                return Err(ParsingError {
                    span: t.span,
                    error_type: ParsingErrorType::NumberTooHigh(n, max),
                });
            } else {
                self.data.push(n as u16);
            }
        }
        if let Some(t) = self.lexer.peek().cloned().transpose()?
            && t.token_type == TokenType::Comma
        {
            self.lexer.next();
            self.data_add_number(min, max)?;
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
#[error("Erro ao ler {span:?}: {error_type}")]
pub struct ParsingError {
    pub span: Span,
    #[source]
    pub error_type: ParsingErrorType,
}

#[derive(Error, Debug)]
pub enum ParsingErrorType {
    #[error(transparent)]
    TokenError(TokenizerErrorType),
    #[error("Esperava {1:?}, mas foi encontrado {0:?}")]
    UnexpectedToken(Token, TokenType),
    #[error("Sessão {0} não reconhecida. Tente começar com .data ou .text")]
    UnrecognizedSession(String),
    #[error("Fim inesperado do conteúdo")]
    UnexpectedEnd,
    #[error("Número {0} baixo demais para o limite {1}")]
    NumberTooLow(isize, isize),
    #[error("Número {0} alto demais para o limite {1}")]
    NumberTooHigh(isize, isize),
    #[error("Diretiva não reconhecida")]
    UnsupportedDirective,
}

impl From<TokenizerError> for ParsingError {
    fn from(value: TokenizerError) -> Self {
        ParsingError {
            span: value.span,
            error_type: ParsingErrorType::TokenError(value.error_type),
        }
    }
}

/*
program :: section+

section :: data | text

data :: ".data" (datadef ;*)*

datadef :: identifier : datavalue

datavalue :: singlevalue | multiplevalue

singlevalue :: directive value

multiplevalue :: directive value (, value)*

value :: valuesimple | string

valuesimple :: int | char

text :: ".text" (instruction ;*)*

instruction :: (identifier :)* keyword (valuesimple | identifier)
*/
