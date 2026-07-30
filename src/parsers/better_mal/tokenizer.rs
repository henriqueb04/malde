use std::fmt::Display;

use thiserror::Error;

use crate::architecture::datapath::Register;
use crate::parsers::source_map::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Identifier,
    Semicolon,
    Colon,
    Comma,
    LeftParen,
    RightParen,
    // bool false para lshift, bool true para rshift
    Shift(u8),
    AluFunc(u8),
    Register(Register),
    If,
    Then,
    Goto,
    Plus,
    Assign,
    Newline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
}

pub struct Tokenizer<'a> {
    source_map: SourceMap,
    reader: SourceReader<'a>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(source_map: &'a SourceMap) -> Self {
        Tokenizer {
            source_map: source_map.clone(),
            reader: source_map.reader(),
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
                '-' => {
                    return Some(Err(TokenizerError {
                        span: Span {
                            start,
                            end: start + len,
                            line,
                            col,
                        },
                        error_type: TokenizerErrorType::UnexpectedMinus,
                    }));
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
                '0' => {
                    self.reader.next();
                    Some(Token {
                        token_type: TokenType::Register(Register::Zero),
                        span: Span {
                            start,
                            end: start + len,
                            line,
                            col,
                        },
                    })
                }
                '1' => {
                    self.reader.next();
                    Some(Token {
                        token_type: TokenType::Register(Register::One),
                        span: Span {
                            start,
                            end: start + len,
                            line,
                            col,
                        },
                    })
                }
                _ if is_identifier_start(&c) => {
                    let span = self.read_identifier()?;
                    let name = self.source_map.get_span(&span);
                    if let Ok(register) = name.parse::<Register>() {
                        Some(Token {
                            token_type: TokenType::Register(register),
                            span,
                        })
                    } else {
                        Some(Token {
                            token_type: get_keyword(name),
                            span,
                        })
                    }
                }
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
    c.is_alphabetic() || *c == '_'
}
fn is_identifier_body(c: &char) -> bool {
    c.is_alphanumeric() || *c == '_'
}

fn get_keyword(s: &str) -> TokenType {
    match s {
        "lshift" => TokenType::Shift(1),
        "rshift" => TokenType::Shift(2),
        "band" => TokenType::AluFunc(1),
        "inv" => TokenType::AluFunc(3),
        "if" => TokenType::If,
        "then" => TokenType::Then,
        "goto" => TokenType::Goto,
        _ => TokenType::Identifier,
    }
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
            }
        )
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

    fn lexer_assert(lexer: &mut Tokenizer) -> impl FnMut(TokenType, &str) {
        move |typ, content| {
            let token = lexer.next().unwrap().unwrap();
            assert_eq!(token.token_type, typ);
            assert_eq!(lexer.source_map.get_span(&token.span), content);
        }
    }

    #[test]
    fn test_tokens() {
        let source_map = SourceMap::from_content(
            "
        alu pc ac sp ir tir 0 1 (-1) amask smask a b c d e f
        LABEL: alu := lshift(a + b); mar := rshift(inv(mbr));
        ",
        );
        let mut lexer = Tokenizer::new(&source_map);
        let mut assert_next = lexer_assert(&mut lexer);
        assert_next(TokenType::Newline, "\n");
        assert_next(TokenType::Register(Register::Alu), "alu");
        assert_next(TokenType::Register(Register::Pc), "pc");
        assert_next(TokenType::Register(Register::Ac), "ac");
        assert_next(TokenType::Register(Register::Sp), "sp");
        assert_next(TokenType::Register(Register::Ir), "ir");
        assert_next(TokenType::Register(Register::Tir), "tir");
        assert_next(TokenType::Register(Register::Zero), "0");
        assert_next(TokenType::Register(Register::One), "1");
        assert_next(TokenType::Register(Register::MinusOne), "(-1)");
        assert_next(TokenType::Register(Register::Amask), "amask");
        assert_next(TokenType::Register(Register::Smask), "smask");
        assert_next(TokenType::Register(Register::A), "a");
        assert_next(TokenType::Register(Register::B), "b");
        assert_next(TokenType::Register(Register::C), "c");
        assert_next(TokenType::Register(Register::D), "d");
        assert_next(TokenType::Register(Register::E), "e");
        assert_next(TokenType::Register(Register::F), "f");
        assert_next(TokenType::Newline, "\n");
        assert_next(TokenType::Identifier, "LABEL");
        assert_next(TokenType::Colon, ":");
        assert_next(TokenType::Register(Register::Alu), "alu");
        assert_next(TokenType::Assign, ":=");
        assert_next(TokenType::Shift(1), "lshift");
        assert_next(TokenType::LeftParen, "(");
        assert_next(TokenType::Register(Register::A), "a");
        assert_next(TokenType::Plus, "+");
        assert_next(TokenType::Register(Register::B), "b");
        assert_next(TokenType::RightParen, ")");
        assert_next(TokenType::Semicolon, ";");
        assert_next(TokenType::Register(Register::Mar), "mar");
        assert_next(TokenType::Assign, ":=");
        assert_next(TokenType::Shift(2), "rshift");
        assert_next(TokenType::LeftParen, "(");
        assert_next(TokenType::AluFunc(3), "inv");
        assert_next(TokenType::LeftParen, "(");
        assert_next(TokenType::Register(Register::Mbr), "mbr");
        assert_next(TokenType::RightParen, ")");
        assert_next(TokenType::RightParen, ")");
        assert_next(TokenType::Semicolon, ";");
    }
}
