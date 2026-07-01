use std::{collections::HashMap, iter::Peekable, mem::discriminant};

use log::{debug, warn};
use thiserror::Error;

use crate::parsers::better_asm::tokenizer::{
    Token, TokenType, Tokenizer, TokenizerError, TokenizerErrorType,
};
use crate::parsers::source_map::{SourceMap, Span};

pub const DEFAULT_KEYWORDS: [(&str, &str); 24] = [
    ("LODD", "0000"),
    ("STOD", "0001"),
    ("ADDD", "0010"),
    ("SUBD", "0011"),
    ("JPOS", "0100"),
    ("JZER", "0101"),
    ("JUMP", "0110"),
    ("LOCO", "0111"),
    ("LODL", "1000"),
    ("STOL", "1001"),
    ("ADDL", "1010"),
    ("SUBL", "1011"),
    ("JNEG", "1100"),
    ("JNZE", "1101"),
    ("CALL", "1110"),
    ("PSHI", "1111000000000000"),
    ("POPI", "1111001000000000"),
    ("PUSH", "1111010000000000"),
    ("POP", "1111011000000000"),
    ("RETN", "1111100000000000"),
    ("SWAP", "1111101000000000"),
    ("HALT", "0000000000000000"),
    ("INSP", "11111100"),
    ("DESP", "11111110"),
];

pub struct ASMParser<'a> {
    source_map: SourceMap<'a>,
    keywords: HashMap<String, String>,
    data_offset: usize,
    lexer: Peekable<Tokenizer<'a>>,
    data: Vec<u16>,
    ins: Vec<u16>,
    data_mappings: HashMap<&'a str, usize>,
    ins_mappings: HashMap<&'a str, usize>,
    pre_ins: Vec<PreInstruction<'a>>,
}

impl<'a> ASMParser<'a> {
    pub fn new(
        source_map: SourceMap<'a>,
        keywords: HashMap<String, String>,
        data_offset: usize,
    ) -> Self {
        ASMParser {
            source_map: source_map.clone(),
            keywords,
            data_offset,
            lexer: Tokenizer::new(&source_map).peekable(),
            data: Vec::new(),
            ins: Vec::new(),
            data_mappings: HashMap::new(),
            ins_mappings: HashMap::new(),
            pre_ins: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Result<(Vec<u16>, Vec<u16>), ParsingError> {
        while self.lexer.peek().cloned().transpose()?.is_some() {
            self.read_section()?;
        }
        self.validate_pre_ins()?;
        Ok((self.ins, self.data))
    }

    fn validate_pre_ins(&mut self) -> Result<(), ParsingError> {
        for mut p in self.pre_ins.clone().into_iter() {
            if p.argument != PreInstructionArg::None {
                let arg_size = 16 - p.keyword_bin.len() as isize;
                let arg_min: isize = -((1 << (arg_size - 1)) - 1);
                let arg_max: isize = (1 << arg_size) - 1;
                let (span, n): (Span, isize) = match p.argument {
                    PreInstructionArg::Label(span, s) => {
                        let n = self.get_mapping(s).map_err(|err| ParsingError {
                            span: span.clone(),
                            error_type: err,
                        })? as isize;
                        (span, n)
                    }
                    PreInstructionArg::Int(span, n) => (span, n),
                    _ => Default::default(),
                };
                if (arg_min..=arg_max).contains(&n) {
                    p.keyword_bin.push_str(&(format!("{:016b}", n))[(16-arg_size as usize)..16])
                } else {
                    return Err(ParsingError {
                        span,
                        error_type: if n < arg_min {
                            ParsingErrorType::NumberTooLow(n, arg_min)
                        } else {
                            ParsingErrorType::NumberTooHigh(n, arg_max)
                        },
                    });
                }
            }
            self.ins
                .push(u16::from_str_radix(&p.keyword_bin, 2).map_err(|_| ParsingError {
                    span: p.keyword_span,
                    error_type: ParsingErrorType::InvalidInstruction(p.keyword_bin),
                })?);
        }
        Ok(())
    }

    fn read_section(&mut self) -> Result<(), ParsingError> {
        let t = self.expect(TokenType::Directive)?;
        let sec = self.source_map.get_span(&t.span);
        match sec {
            ".data" => self.read_data()?,
            ".text" => self.read_text()?,
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
        self.data_add_mapping(self.source_map.get(&t), data_start)
            .map_err(|err| ParsingError {
                span: t.span,
                error_type: err,
            })?;
        self.burn_semicolons()?;
        if let Some(t) = self.lexer.peek().cloned().transpose()?
            && t.token_type == TokenType::Identifier
        {
            self.read_data()?;
        }
        Ok(())
    }

    fn read_text(&mut self) -> Result<(), ParsingError> {
        let t1 = self.expect(TokenType::Identifier)?;
        let c1 = self.source_map.get_span(&t1.span);
        let t2 = self.lexer.peek().cloned().transpose()?;
        println!("{}", c1);
        if let Some(t2) = t2
            && t2.token_type == TokenType::Colon
        {
            println!("label");
            // LABEL:
            if let Err(err) = self.text_add_mapping(self.source_map.get(&t1), self.pre_ins.len())
                .map_err(|err| ParsingError {
                    span: t1.span,
                    error_type: err,
                }) {
                    println!("This should not happen {}", err);
                    return Err(err);
                }
            self.lexer.next();
            self.read_text()?;
        } else if let Some(keyword_bin) = self.keywords.get(c1).cloned() {
            println!("inst");
            if keyword_bin.len() == 16 {
                // 16 bit instruction
                self.pre_ins.push(PreInstruction {
                    keyword_span: t1.span.clone(),
                    keyword_bin,
                    argument: PreInstructionArg::None,
                });
            } else {
                let t2 = self.next()?;
                match t2.token_type {
                    TokenType::Identifier => self.pre_ins.push(PreInstruction {
                        keyword_span: t1.span.clone(),
                        keyword_bin,
                        argument: PreInstructionArg::Label(
                            t2.span.clone(),
                            self.source_map.get(&t2),
                        ),
                    }),

                    TokenType::Int(n) => self.pre_ins.push(PreInstruction {
                        keyword_span: t1.span.clone(),
                        keyword_bin,
                        argument: PreInstructionArg::Int(t2.span.clone(), n),
                    }),
                    _ => {
                        return Err(ParsingError {
                            span: t2.span.clone(),
                            error_type: ParsingErrorType::UnexpectedToken(
                                t2,
                                TokenType::Identifier,
                            ),
                        });
                    }
                }
            }
        } else {
            return Err(ParsingError {
                error_type: ParsingErrorType::UnrecognizedKeyword(
                    self.source_map.get_span(&t1.span).to_string(),
                ),
                span: t1.span,
            });
        }
        self.burn_semicolons()?;
        if let Some(t) = self.lexer.peek().cloned().transpose()?
            && t.token_type == TokenType::Identifier
        {
            self.read_text()?;
        }
        Ok(())
    }

    fn burn_semicolons(&mut self) -> Result<(), ParsingError> {
        while let Some(t) = self.lexer.peek().cloned().transpose()?
            && t.token_type == TokenType::Semicolon
        {
            self.lexer.next();
        }
        Ok(())
    }

    fn next(&mut self) -> Result<Token, ParsingError> {
        let t = self.lexer.next().ok_or(ParsingError {
            span: self.source_map.end(),
            error_type: ParsingErrorType::UnexpectedEnd,
        })??;
        Ok(t)
    }
    fn expect(&mut self, typ: TokenType) -> Result<Token, ParsingError> {
        let t = self.next()?;
        if discriminant(&t.token_type) != discriminant(&typ) {
            return Err(ParsingError {
                span: t.span.clone(),
                error_type: ParsingErrorType::UnexpectedToken(t, typ),
            });
        }
        Ok(t)
    }

    fn get_mapping(&self, label: &'a str) -> Result<usize, ParsingErrorType> {
        let Some(n) = self
            .data_mappings
            .get(label)
            .map(|v| v + self.data_offset)
            .or(self.ins_mappings.get(label).copied())
            else {
                return Err(ParsingErrorType::UnrecognizedLabel(label.to_string()));
            };
        Ok(n)
    }
    fn data_add_mapping(&mut self, label: &'a str, addr: usize) -> Result<(), ParsingErrorType> {
        if self
            .data_mappings
            .get(label)
            .or_else(|| self.ins_mappings.get(label))
            .is_some()
        {
            return Err(ParsingErrorType::DuplicatedLabel(label.to_string()));
        }
        self.data_mappings.insert(label, addr);
        Ok(())
    }
    fn text_add_mapping(&mut self, label: &'a str, addr: usize) -> Result<(), ParsingErrorType> {
        if self
            .data_mappings
            .get(label)
            .or_else(|| self.ins_mappings.get(label))
            .is_some()
        {
            return Err(ParsingErrorType::DuplicatedLabel(label.to_string()));
        }
        self.ins_mappings.insert(label, addr);
        Ok(())
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

    fn text_add_ins(&mut self, content: String) -> Result<(), ParsingErrorType> {
        let Ok(n) = u16::from_str_radix(&content, 2) else {
            return Err(ParsingErrorType::InvalidInstruction(content));
        };
        self.ins.push(n);
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
    #[error("Keyword \"{0}\" não reconhecida")]
    UnrecognizedKeyword(String),
    #[error("Rótudo \"{0}\" não reconhecido")]
    UnrecognizedLabel(String),
    #[error("Fim inesperado do conteúdo")]
    UnexpectedEnd,
    #[error("Número {0} baixo demais para o limite {1}")]
    NumberTooLow(isize, isize),
    #[error("Número {0} alto demais para o limite {1}")]
    NumberTooHigh(isize, isize),
    #[error("Diretiva não reconhecida")]
    UnsupportedDirective,
    #[error("Instrução {0} inválida")]
    InvalidInstruction(String),
    #[error("Rótulo {0} já está em uso")]
    DuplicatedLabel(String),
}

impl From<TokenizerError> for ParsingError {
    fn from(value: TokenizerError) -> Self {
        ParsingError {
            span: value.span,
            error_type: ParsingErrorType::TokenError(value.error_type),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreInstructionArg<'a> {
    Label(Span, &'a str),
    Int(Span, isize),
    None,
}

#[derive(Debug, Clone)]
struct PreInstruction<'a> {
    keyword_span: Span,
    keyword_bin: String,
    argument: PreInstructionArg<'a>,
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

#[cfg(test)]
mod tests {
    use crate::virtual_machine::DATA_SEGMENT_START;

    use super::*;
    use log::debug;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_data() {
        let def_keys =
            HashMap::from(DEFAULT_KEYWORDS.map(|(a, b)| (String::from(a), String::from(b))));
        let source_map = SourceMap {
            filename: "",
            content: ".data
                TESTE1: .word 1;
                TESTE2: .word 1,2
                TESTE3: .asciz \"St\n \\na\"
                TESTE4: .byte 1,2,3,4;;;
                TESTE5: .byte 'a', '\\n'
            .text
                    LOCO 5
            MAIN:   ADDL -1
                    SWAP
                    LODD TESTE3
                    INSP -1
            MAIN2:
                    JUMP MAIN
",
        };
        let parser = ASMParser::new(source_map.clone(), def_keys, DATA_SEGMENT_START);
        let (ins, data) = parser.parse().unwrap();
        let expected = [
            1u16,
            1,
            2,
            'S' as u16,
            't' as u16,
            '\n' as u16,
            ' ' as u16,
            '\n' as u16,
            'a' as u16,
            0,
            1,
            2,
            3,
            4,
            'a' as u16,
            '\n' as u16,
        ];
        assert_eq!(data, expected);
        assert_eq!(
            ins,
            [
                0b0111000000000101,
                0b1010111111111111,
                0b1111101000000000,
                0b0000000000000011 + DATA_SEGMENT_START as u16,
                0b1111110011111111,
                0b0110000000000001
            ]
        );
    }
}
