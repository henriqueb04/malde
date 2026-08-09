use std::fmt::Display;

use thiserror::Error;

use crate::{architecture::datapath::Register, parsers::source_map::*};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Identifier,
    Semicolon,
    Colon,
    Comma,
    LeftParen,
    RightParen,
    Shift(u8),
    AluFunc(u8),
    Register(Register),
    If,
    Then,
    Goto,
    Plus,
    Assign,
    Syscall,
    Wr,
    Rd,
    Condition(u8),
    Newline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
}

pub struct Tokenizer<'a> {
    reader: SourceReader<'a>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(source_map: &'a SourceMap) -> Self {
        Tokenizer {
            reader: source_map.reader(),
        }
    }

    pub fn map_identifier(s: &str) -> TokenType {
        if let Ok(register) = s.parse::<Register>() {
            TokenType::Register(register)
        } else {
            match s {
                "lshift" => TokenType::Shift(1),
                "rshift" => TokenType::Shift(2),
                "band" => TokenType::AluFunc(1),
                "inv" => TokenType::AluFunc(3),
                "if" => TokenType::If,
                "then" => TokenType::Then,
                "goto" => TokenType::Goto,
                "syscall" => TokenType::Syscall,
                "wr" => TokenType::Wr,
                "rd" => TokenType::Rd,
                "n" => TokenType::Condition(1),
                "z" => TokenType::Condition(2),
                _ => TokenType::Identifier,
            }
        }
    }

    fn read_identifier(&mut self) -> Option<Span> {
        let start = self.reader.offset();
        let mut end = self.reader.offset();
        let line = self.reader.line();
        let col = self.reader.col();
        while let Some((_, &c)) = self.reader.peek()
            && is_identifier_body(&c)
        {
            self.reader.next();
            end += c.len_utf8();
        }
        if start != end {
            let span = Span {
                start,
                end,
                line,
                col,
            };
            Some(span)
        } else {
            None
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token, TokenizerError>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut comment = false;
        while let Some((len, &c)) = self.reader.peek() {
            let start = self.reader.offset();
            let line = self.reader.line();
            let col = self.reader.col();
            let t: Option<Token> = match c {
                '#' => {
                    comment = true;
                    self.reader.next();
                    None
                }
                '\\' => {
                    self.reader.next();
                    let start2 = self.reader.offset();
                    let line2 = self.reader.line();
                    let col2 = self.reader.col();
                    if let Some((len2, c2)) = self.reader.next() {
                        if c2 == '\n' {
                            comment = false;
                            None
                        } else if !comment {
                            return Some(Err(TokenizerError {
                                span: Span {
                                    start: start2,
                                    end: start2 + len2,
                                    line: line2,
                                    col: col2,
                                },
                                error_type: TokenizerErrorType::UnexpectedCharacter,
                            }));
                        } else {
                            None
                        }
                    } else if !comment {
                        return Some(Err(TokenizerError {
                            span: Span {
                                start,
                                end: start + len,
                                line,
                                col,
                            },
                            error_type: TokenizerErrorType::UnexpectedCharacter,
                        }));
                    } else {
                        None
                    }
                }
                '\n' => {
                    comment = false;
                    self.reader.next();
                    Some(Token {
                        token_type: TokenType::Newline,
                        span: Span {
                            start,
                            end: self.reader.offset(),
                            line,
                            col,
                        },
                    })
                }
                _ if comment => {
                    self.reader.next();
                    None
                }
                _ if c.is_whitespace() => {
                    self.reader.next();
                    None
                }
                '(' => {
                    self.reader.next();
                    if let Some((len2, c2)) = self.reader.peek()
                        && *c2 == '-'
                    {
                        let start2 = self.reader.offset();
                        self.reader.next();
                        if self.reader.next().map(|o| o.1) == Some('1')
                            && self.reader.next().map(|o| o.1) == Some(')')
                        {
                            Some(Token {
                                token_type: TokenType::Register(Register::MinusOne),
                                span: Span {
                                    start,
                                    end: self.reader.offset(),
                                    line,
                                    col,
                                },
                            })
                        } else {
                            return Some(Err(TokenizerError {
                                span: Span {
                                    start: start2,
                                    end: start2 + len2,
                                    line,
                                    col,
                                },
                                error_type: TokenizerErrorType::UnexpectedMinus,
                            }));
                        }
                    } else {
                        Some(Token {
                            token_type: TokenType::LeftParen,
                            span: Span {
                                start,
                                end: start + len,
                                line,
                                col,
                            },
                        })
                    }
                }
                ')' => {
                    self.reader.next();
                    Some(Token {
                        token_type: TokenType::RightParen,
                        span: Span {
                            start,
                            end: start + len,
                            line,
                            col,
                        },
                    })
                }
                '+' => {
                    self.reader.next();
                    Some(Token {
                        token_type: TokenType::Plus,
                        span: Span {
                            start,
                            end: self.reader.offset(),
                            line,
                            col,
                        },
                    })
                }
                _ if is_identifier_start(&c) => Some(Token {
                    token_type: TokenType::Identifier,
                    span: self.read_identifier()?,
                }),
                ';' => {
                    self.reader.next();
                    let span = Span {
                        start,
                        end: self.reader.offset(),
                        line,
                        col,
                    };
                    Some(Token {
                        token_type: TokenType::Semicolon,
                        span,
                    })
                }
                ':' => {
                    self.reader.next();
                    let start2 = self.reader.offset();
                    if let Some((len2, &c2)) = self.reader.peek()
                        && c2 == '='
                    {
                        self.reader.next();
                        Some(Token {
                            token_type: TokenType::Assign,
                            span: Span {
                                start,
                                end: start2 + len2,
                                line,
                                col,
                            },
                        })
                    } else {
                        Some(Token {
                            token_type: TokenType::Colon,
                            span: Span {
                                start,
                                end: start + len,
                                line,
                                col,
                            },
                        })
                    }
                }
                ',' => {
                    self.reader.next();
                    Some(Token {
                        token_type: TokenType::Comma,
                        span: Span {
                            start,
                            end: self.reader.offset(),
                            line,
                            col,
                        },
                    })
                }
                _ => {
                    self.reader.next();
                    return Some(Err(TokenizerError {
                        span: Span {
                            start,
                            end: self.reader.offset(),
                            line,
                            col,
                        },
                        error_type: TokenizerErrorType::UnexpectedCharacter,
                    }));
                }
            };
            if let Some(token) = t {
                return Some(Ok(token));
            }
        }
        None
    }
}

fn is_identifier_start(c: &char) -> bool {
    is_identifier_body(c)
}
fn is_identifier_body(c: &char) -> bool {
    c.is_alphanumeric() || *c == '_' || *c == '-'
}

#[derive(Error, Clone, Debug, PartialEq, Eq)]
#[error("Erro de sintaxe: {error_type} em {span}")]
pub struct TokenizerError {
    pub span: Span,
    #[source]
    pub error_type: TokenizerErrorType,
}

#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum TokenizerErrorType {
    #[error("Caractere inexperado")]
    UnexpectedCharacter,
    #[error("Caractere \"-\" inexperado. (Nota: o nome do registrador é (-1), com parênteses)")]
    UnexpectedMinus,
}

impl Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TokenType::Identifier => "identificador".to_string(),
                TokenType::Semicolon => "ponto e vírgula".to_string(),
                TokenType::Colon => "dois pontos".to_string(),
                TokenType::Comma => "vírgula".to_string(),
                TokenType::Newline => "quebra de linha".to_string(),
                TokenType::LeftParen => "parênseses esquedo".to_string(),
                TokenType::RightParen => "parênseses direito".to_string(),
                TokenType::Shift(sh) => match sh {
                    1 => "lshift",
                    2 => "rshift",
                    _ => "lshift ou rshift",
                }
                .to_string(),
                TokenType::AluFunc(..) => "operação da ula".to_string(),
                TokenType::Register(reg) => format!("registrador {}", reg),
                TokenType::If => "if".to_string(),
                TokenType::Then => "then".to_string(),
                TokenType::Goto => "goto".to_string(),
                TokenType::Plus => "+".to_string(),
                TokenType::Assign => ":=".to_string(),
                TokenType::Syscall => "syscall".to_string(),
                TokenType::Wr => "wr".to_string(),
                TokenType::Rd => "rd".to_string(),
                TokenType::Condition(cond) => match cond {
                    1 => "n",
                    2 => "z",
                    _ => "condição",
                }
                .to_string(),
            }
        )
    }
}

impl Token {
    pub fn mapped(self, source_map: &SourceMap) -> Token {
        let name = source_map.get_span(&self.span);
        let typ = if self.token_type == TokenType::Identifier {
            Tokenizer::map_identifier(name)
        } else {
            self.token_type
        };
        Token {
            token_type: typ,
            span: self.span,
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.token_type, self.span)
    }
}

impl HasSpan for Token {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    impl Tokenizer<'_> {
        fn next_mapped(&mut self, source_map: &SourceMap) -> Option<Result<Token, TokenizerError>> {
            match self.next()? {
                Ok(t) => {
                    if t.token_type == TokenType::Identifier {
                        Some(Ok(Token {
                            token_type: Tokenizer::map_identifier(source_map.get(&t)),
                            span: t.span,
                        }))
                    } else {
                        Some(Ok(t))
                    }
                }
                Err(err) => Some(Err(err)),
            }
        }
    }

    struct Assert<'a> {
        source_map: SourceMap,
        lexer: Tokenizer<'a>,
    }
    impl<'a> Assert<'a> {
        pub fn new(lexer: Tokenizer<'a>, source_map: SourceMap) -> Self {
            Assert { lexer, source_map }
        }
        #[track_caller]
        fn next(&mut self, typ: TokenType, content: &str) {
            let token = self.lexer.next_mapped(&self.source_map).unwrap().unwrap();
            assert_eq!(token.token_type, typ);
            assert_eq!(self.source_map.get(&token), content);
        }
    }

    #[test]
    fn test_tokens() {
        let source_map = SourceMap::from_content(
            "
        alu pc ac sp ir tir 0 1 (-1) amask smask a b c d e f
        LABEL: alu := lshift(a + b); mar := rshift(inv(mbr));
        wr; rd; syscall; if n then goto LABEL;
        LABEL2: if z goto LABEL;
        ",
        );
        let lexer = Tokenizer::new(&source_map);
        let mut assert = Assert::new(lexer, source_map.clone());
        assert.next(TokenType::Newline, "\n");
        assert.next(TokenType::Register(Register::Alu), "alu");
        assert.next(TokenType::Register(Register::Pc), "pc");
        assert.next(TokenType::Register(Register::Ac), "ac");
        assert.next(TokenType::Register(Register::Sp), "sp");
        assert.next(TokenType::Register(Register::Ir), "ir");
        assert.next(TokenType::Register(Register::Tir), "tir");
        assert.next(TokenType::Register(Register::Zero), "0");
        assert.next(TokenType::Register(Register::One), "1");
        assert.next(TokenType::Register(Register::MinusOne), "(-1)");
        assert.next(TokenType::Register(Register::Amask), "amask");
        assert.next(TokenType::Register(Register::Smask), "smask");
        assert.next(TokenType::Register(Register::A), "a");
        assert.next(TokenType::Register(Register::B), "b");
        assert.next(TokenType::Register(Register::C), "c");
        assert.next(TokenType::Register(Register::D), "d");
        assert.next(TokenType::Register(Register::E), "e");
        assert.next(TokenType::Register(Register::F), "f");
        assert.next(TokenType::Newline, "\n");
        assert.next(TokenType::Identifier, "LABEL");
        assert.next(TokenType::Colon, ":");
        assert.next(TokenType::Register(Register::Alu), "alu");
        assert.next(TokenType::Assign, ":=");
        assert.next(TokenType::Shift(1), "lshift");
        assert.next(TokenType::LeftParen, "(");
        assert.next(TokenType::Register(Register::A), "a");
        assert.next(TokenType::Plus, "+");
        assert.next(TokenType::Register(Register::B), "b");
        assert.next(TokenType::RightParen, ")");
        assert.next(TokenType::Semicolon, ";");
        assert.next(TokenType::Register(Register::Mar), "mar");
        assert.next(TokenType::Assign, ":=");
        assert.next(TokenType::Shift(2), "rshift");
        assert.next(TokenType::LeftParen, "(");
        assert.next(TokenType::AluFunc(3), "inv");
        assert.next(TokenType::LeftParen, "(");
        assert.next(TokenType::Register(Register::Mbr), "mbr");
        assert.next(TokenType::RightParen, ")");
        assert.next(TokenType::RightParen, ")");
        assert.next(TokenType::Semicolon, ";");
        assert.next(TokenType::Newline, "\n");
        assert.next(TokenType::Wr, "wr");
        assert.next(TokenType::Semicolon, ";");
        assert.next(TokenType::Rd, "rd");
        assert.next(TokenType::Semicolon, ";");
        assert.next(TokenType::Syscall, "syscall");
        assert.next(TokenType::Semicolon, ";");
        assert.next(TokenType::If, "if");
        assert.next(TokenType::Condition(1), "n");
        assert.next(TokenType::Then, "then");
        assert.next(TokenType::Goto, "goto");
        assert.next(TokenType::Identifier, "LABEL");
        assert.next(TokenType::Semicolon, ";");
        assert.next(TokenType::Newline, "\n");
        assert.next(TokenType::Identifier, "LABEL2");
        assert.next(TokenType::Colon, ":");
        assert.next(TokenType::If, "if");
        assert.next(TokenType::Condition(2), "z");
        assert.next(TokenType::Goto, "goto");
        assert.next(TokenType::Identifier, "LABEL");
        assert.next(TokenType::Semicolon, ";");
        assert.next(TokenType::Newline, "\n");
    }
}
