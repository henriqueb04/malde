use std::fmt::Display;
use std::{collections::HashMap, iter::Peekable, mem::discriminant};

use thiserror::Error;

use crate::parsers::asm::{
    keyword_map::*,
    tokenizer::{Token, TokenType, Tokenizer, TokenizerError, TokenizerErrorType},
};
use crate::parsers::source_map::{SourceMap, Span};

pub struct ASMParser<'a> {
    source_map: &'a SourceMap,
    keywords: KeywordMap,
    data_offset: usize,
    lexer: Peekable<Tokenizer<'a>>,
    data_mem: Vec<u16>,
    ins_mem: Vec<u16>,
    data_mappings: HashMap<&'a str, usize>,
    ins_mappings: HashMap<&'a str, usize>,
    pre_ins: Vec<PreInstruction<'a>>,
    ins_list: Vec<Instruction>,
}

impl<'a> ASMParser<'a> {
    pub fn new(source_map: &'a SourceMap, keywords: KeywordMap, data_offset: usize) -> Self {
        ASMParser {
            source_map,
            keywords,
            data_offset,
            lexer: Tokenizer::new(source_map).peekable(),
            data_mem: Vec::new(),
            ins_mem: Vec::new(),
            data_mappings: HashMap::new(),
            ins_mappings: HashMap::new(),
            pre_ins: Vec::new(),
            ins_list: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Result<ParserResult, ASMParsingError> {
        self.inner_parse()
            .map(|_| ParserResult {
                data_mem: self.data_mem,
                ins_mem: self.ins_mem,
                instructions: self.ins_list,
            })
            .map_err(|err| ASMParsingError::new(self.source_map, err))
    }

    fn inner_parse(&mut self) -> Result<(), ParsingError> {
        while self.peek_kind()?.is_some() {
            self.read_section()?;
        }
        self.process_pre_ins()?;
        Ok(())
    }

    fn process_pre_ins(&mut self) -> Result<(), ParsingError> {
        for p in &mut self.pre_ins.iter() {
            let v = if let Some(argument) = p.argument.as_ref() {
                let arg_size = argument.0;
                let span = &argument.1;
                let arg_min: isize = -(1 << (arg_size - 1));
                let arg_max: isize = (1 << arg_size) - 1;
                let n: isize = match argument.2 {
                    PreInstructionArg::Label(s) => {
                        self.get_mapping(s).map_err(|err| ParsingError {
                            span: span.clone(),
                            error_type: err,
                        })? as isize
                    }
                    PreInstructionArg::Int(n) => n,
                };
                if (arg_min..=arg_max).contains(&n) {
                    let mask = (1 << arg_size) - 1;
                    (p.keyword_val << arg_size) | (n as usize & mask)
                } else {
                    return Err(ParsingError {
                        span: span.clone(),
                        error_type: if n < arg_min {
                            ParsingErrorType::NumberTooLow(n, arg_min)
                        } else {
                            ParsingErrorType::NumberTooHigh(n, arg_max)
                        },
                    });
                }
            } else {
                p.keyword_val
            };
            let instruction = Instruction {
                content: self.source_map.get_line(p.keyword_span.line).to_string(),
                bin: format!("{:016b}", v),
            };
            self.ins_mem.push(v as u16);
            self.ins_list.push(instruction);
        }
        Ok(())
    }

    fn read_section(&mut self) -> Result<(), ParsingError> {
        self.burn_newlines()?;
        let t = self.expect(TokenType::Directive).map_err(|err| {
            if let ParsingErrorType::UnexpectedToken(t, ..) = err.error_type {
                ParsingError {
                    span: err.span,
                    error_type: ParsingErrorType::NotASection(t),
                }
            } else {
                err
            }
        })?;
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
        self.burn_newlines()?;
        Ok(())
    }

    fn read_data(&mut self) -> Result<(), ParsingError> {
        self.burn_newlines()?;
        while let Some(t) = self.peek_kind()?
            && *t == TokenType::Identifier
        {
            self.burn_newlines()?;
            let t = self.expect(TokenType::Identifier)?;
            self.expect(TokenType::Colon)?;
            self.burn_newlines()?;
            let dir = self.expect(TokenType::Directive)?;
            self.burn_newlines()?;
            let data_start = self.data_mem.len();
            let dir_source = self.source_map.get_span(&dir.span);
            match dir_source {
                ".ascii" => self.data_add_string()?,
                ".asciz" | ".asciiz" => {
                    self.burn_newlines()?;
                    self.data_add_string()?;
                    self.data_mem.push(0);
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
                        self.data_mem
                            .resize(self.data_mem.len() + (n as usize).div_ceil(2), 0);
                    }
                }
                _ => {
                    return Err(ParsingError {
                        error_type: ParsingErrorType::UnsupportedDirective(dir_source.to_string()),
                        span: dir.span,
                    });
                }
            }
            self.data_add_mapping(self.source_map.get(&t), data_start)
                .map_err(|err| ParsingError {
                    span: t.span,
                    error_type: err,
                })?;
            self.burn_semicolons()?;
            self.expect_newline()?;
        }
        Ok(())
    }

    fn read_text(&mut self) -> Result<(), ParsingError> {
        self.burn_newlines()?;
        while let Some(t) = self.peek_kind()?
            && *t == TokenType::Identifier
        {
            self.burn_newlines()?;
            let t1 = self.expect(TokenType::Identifier)?;
            let c1 = self.source_map.get_span(&t1.span);
            let t2 = self.peek_kind()?;
            if let Some(t2) = t2
                && *t2 == TokenType::Colon
            {
                // LABEL:
                self.text_add_mapping(self.source_map.get(&t1), self.pre_ins.len())
                    .map_err(|err| ParsingError {
                        span: t1.span,
                        error_type: err,
                    })?;
                self.lexer.next();
                self.read_text()?;
            } else if let Some((keyword_val, arg_size)) = self.keywords.get(c1).cloned() {
                if arg_size == 0 {
                    // 16 bit instruction
                    self.pre_ins.push(PreInstruction {
                        keyword_span: t1.span.clone(),
                        keyword_val,
                        argument: None,
                    });
                } else {
                    let t2 = self.next()?;
                    match t2.token_type {
                        TokenType::Identifier => self.pre_ins.push(PreInstruction {
                            keyword_span: t1.span.clone(),
                            keyword_val,
                            argument: Some((
                                arg_size,
                                t2.span.clone(),
                                PreInstructionArg::Label(self.source_map.get(&t2)),
                            )),
                        }),

                        TokenType::Int(n) => self.pre_ins.push(PreInstruction {
                            keyword_span: t1.span.clone(),
                            keyword_val,
                            argument: Some((arg_size, t2.span.clone(), PreInstructionArg::Int(n))),
                        }),
                        _ => {
                            return Err(ParsingError {
                                span: t2.span.clone(),
                                error_type: ParsingErrorType::NotAnArgument(t2),
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
            self.expect_newline()?;
        }
        Ok(())
    }

    fn burn_tokens(&mut self, typ: TokenType) -> Result<(), ParsingError> {
        while let Some(t) = self.peek_kind()?
            && *t == typ
        {
            self.lexer.next();
        }
        Ok(())
    }
    fn burn_semicolons(&mut self) -> Result<(), ParsingError> {
        self.burn_tokens(TokenType::Semicolon)
    }
    fn burn_newlines(&mut self) -> Result<(), ParsingError> {
        self.burn_tokens(TokenType::Newline)
    }

    fn next(&mut self) -> Result<Token, ParsingError> {
        let t = self.lexer.next().ok_or(ParsingError {
            span: self.source_map.end(),
            error_type: ParsingErrorType::UnexpectedEnd,
        })??;
        Ok(t)
    }
    fn peek_kind(&mut self) -> Result<Option<&TokenType>, ParsingError> {
        if let Some(r) = self.lexer.peek() {
            Ok(Some(
                r.as_ref()
                    .map(|t| &t.token_type)
                    .map_err(|err| err.clone())?,
            ))
        } else {
            Ok(None)
        }
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
    fn expect_string(&mut self) -> Result<Token, ParsingError> {
        let t = self.next()?;
        if matches!(t.token_type, TokenType::String(..)) {
            Ok(t)
        } else {
            Err(ParsingError {
                span: t.span.clone(),
                error_type: ParsingErrorType::UnexpectedToken(t, TokenType::String(String::new())),
            })
        }
    }
    fn expect_newline(&mut self) -> Result<(), ParsingError> {
        if self.lexer.peek().is_some() {
            self.expect(TokenType::Newline)?;
        }
        while let Some(t) = self.peek_kind()?
            && (*t == TokenType::Newline || *t == TokenType::Semicolon)
        {
            self.lexer.next();
        }
        Ok(())
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
        let s = self.expect_string()?;
        if let TokenType::String(seq) = s.token_type {
            for (_, c) in seq.char_indices() {
                self.data_mem.push(c as u16);
            }
        }
        Ok(())
    }
    fn data_add_number(&mut self, min: isize, max: isize) -> Result<(), ParsingError> {
        self.burn_newlines()?;
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
                self.data_mem.push(n as u16);
            }
        }
        if let Some(t) = self.peek_kind()?
            && *t == TokenType::Comma
        {
            self.burn_newlines()?;
            self.lexer.next();
            self.data_add_number(min, max)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ParsingError {
    pub span: Span,
    pub error_type: ParsingErrorType,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub struct ASMParsingError {
    display: String,
    pub span: Span,
    pub error_type: ParsingErrorType,
}

impl ASMParsingError {
    fn new(source_map: &SourceMap, parsing_error: ParsingError) -> Self {
        let display = format!(
            "Erro ao ler linha {}, coluna {}:\n{}\n\n{}",
            parsing_error.span.line,
            parsing_error.span.col,
            source_map.highlight_in_line(&parsing_error.span),
            parsing_error.error_type
        );
        ASMParsingError {
            display,
            span: parsing_error.span,
            error_type: parsing_error.error_type,
        }
    }
}

impl Display for ASMParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

#[derive(Debug, Default)]
pub struct ParserResult {
    pub data_mem: Vec<u16>,
    pub ins_mem: Vec<u16>,
    pub instructions: Vec<Instruction>,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParsingErrorType {
    #[error(transparent)]
    TokenError(TokenizerErrorType),
    #[error("Esperava {1}, mas foi encontrado {0}")]
    UnexpectedToken(Token, TokenType),
    #[error("Sessão {0} não reconhecida. Tente começar com .data ou .text")]
    UnrecognizedSession(String),
    #[error("Keyword \"{0}\" não reconhecida")]
    UnrecognizedKeyword(String),
    #[error("Rótulo \"{0}\" não reconhecido")]
    UnrecognizedLabel(String),
    #[error("Fim inesperado do conteúdo")]
    UnexpectedEnd,
    #[error("Número {0} baixo demais para o limite {1}")]
    NumberTooLow(isize, isize),
    #[error("Número {0} alto demais para o limite {1}")]
    NumberTooHigh(isize, isize),
    #[error("Diretiva não reconhecida")]
    UnsupportedDirective(String),
    #[error("Rótulo {0} já está em uso")]
    DuplicatedLabel(String),
    #[error(
        "Esperava início de seção (.data ou .text), mas foi encontrado {0}. Verifique a formatação..."
    )]
    NotASection(Token),
    #[error("Esperava um inteiro ou rótulo, mas foi encontrado {0}")]
    NotAnArgument(Token),
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
    Label(&'a str),
    Int(isize),
}

#[derive(Debug, Clone)]
struct PreInstruction<'a> {
    keyword_span: Span,
    keyword_val: usize,
    argument: Option<(usize, Span, PreInstructionArg<'a>)>,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub content: String,
    pub bin: String,
}

#[cfg(test)]
mod tests {
    use crate::virtual_machine::DATA_SEGMENT_START;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_empty() {
        let def_keys = KeywordMap::default();
        let source_map = SourceMap::from_content(
            ".data
            .text
            ",
        );
        let parser = ASMParser::new(&source_map, def_keys, DATA_SEGMENT_START);
        let ParserResult {
            data_mem: data,
            ins_mem: ins,
            ..
        } = parser.parse().unwrap();
        assert!(data.is_empty());
        assert!(ins.is_empty());
    }

    #[test]
    fn test_data() {
        let def_keys = KeywordMap::default();
        let source_map = SourceMap::from_content(
            ".data
                TESTE1: \n.word 1;
                TESTE2: .word 1,\n2
                ;;;
                TESTE3: .asciz\n \"St\n \\na\"
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
        );
        let tokens: Result<Vec<(usize, TokenType)>, TokenizerErrorType> =
            Tokenizer::new(&source_map)
                .collect::<Result<Vec<Token>, TokenizerError>>()
                .map(|v| {
                    v.into_iter()
                        .map(|t| (t.span.line, t.token_type))
                        .collect::<Vec<_>>()
                })
                .map_err(|err| err.error_type);
        println!("{:?}", tokens);
        let parser = ASMParser::new(&source_map, def_keys, DATA_SEGMENT_START);
        let ParserResult {
            data_mem: data,
            ins_mem: ins,
            ..
        } = parser.parse().unwrap();
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
        let expected = [
            0b0111000000000101,
            0b1010111111111111,
            0b1111101000000000,
            0b0000000000000011 + DATA_SEGMENT_START as u16,
            0b1111110011111111,
            0b0110000000000001,
        ];
        assert_eq!(ins, expected);
    }

    fn assert_err(content: &str, err: ParsingErrorType) {
        let def_keys = KeywordMap::default();
        let sm = SourceMap::from_content(content);
        let parser = ASMParser::new(&sm, def_keys, DATA_SEGMENT_START);
        assert_eq!(parser.parse().unwrap_err().error_type, err);
    }

    #[test]
    fn test_errors() {
        assert_err(
            "5",
            ParsingErrorType::NotASection(Token {
                token_type: TokenType::Int(5),
                span: Span {
                    start: 0,
                    end: 1,
                    line: 1,
                    col: 1,
                },
            }),
        );
        // FIXME: esse teste merece ser revisado e reimplementado depois
        // assert_err(
        //     ".data\n :",
        //     ParsingErrorType::UnexpectedToken(
        //         Token {
        //             token_type: TokenType::Colon,
        //             span: Span {
        //                 start: 7,
        //                 end: 8,
        //                 line: 2,
        //                 col: 2,
        //             },
        //         },
        //         TokenType::Identifier,
        //     ),
        // );
        assert_err(
            ".data\n TESTE1: .word 1\n,1",
            ParsingErrorType::NotASection(Token {
                token_type: TokenType::Comma,
                span: Span {
                    start: 23,
                    end: 24,
                    line: 3,
                    col: 1,
                },
            }),
        );
        assert_err(
            ".text TESTE1:\n INSP\n 1",
            ParsingErrorType::NotAnArgument(Token {
                token_type: TokenType::Newline,
                span: Span {
                    start: 19,
                    end: 20,
                    line: 2,
                    col: 6,
                },
            }),
        );
        assert_err(
            ".teste",
            ParsingErrorType::UnrecognizedSession(".teste".to_string()),
        );
        assert_err(
            ".text KEYWORD",
            ParsingErrorType::UnrecognizedKeyword("KEYWORD".to_string()),
        );
        assert_err(
            ".text LODD LABEL",
            ParsingErrorType::UnrecognizedLabel("LABEL".to_string()),
        );
        assert_err(".data TESTE", ParsingErrorType::UnexpectedEnd);
        assert_err(
            ".text INSP -99999999",
            ParsingErrorType::NumberTooLow(-99999999, 0b10000000u8 as i8 as isize),
        );
        assert_err(
            ".data VAR: .byte 256",
            ParsingErrorType::NumberTooHigh(256, 255),
        );
        assert_err(
            ".data VAR: .half 55",
            ParsingErrorType::UnsupportedDirective(".half".to_string()),
        );
        assert_err(
            ".data VAR: .half 55",
            ParsingErrorType::UnsupportedDirective(".half".to_string()),
        );
        assert_err(
            ".text LABEL1: SWAP\n LABEL2: SWAP\n LABEL1: SWAP",
            ParsingErrorType::DuplicatedLabel("LABEL1".to_string()),
        );
        assert_err(
            ".text MAIN: LODD .byte",
            ParsingErrorType::NotAnArgument(Token {
                token_type: TokenType::Directive,
                span: Span {
                    start: 17,
                    end: 22,
                    line: 1,
                    col: 18,
                },
            }),
        );
    }
}
