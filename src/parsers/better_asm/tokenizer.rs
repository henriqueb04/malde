use std::fmt::Display;

use thiserror::Error;

use crate::parsers::source_map::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Identifier,
    Directive,
    Semicolon,
    Colon,
    Comma,
    String(String),
    Int(isize),
    Newline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", match self.token_type {
            TokenType::Identifier => "identificador".to_string(),
            TokenType::Directive => "diretiva".to_string(),
            TokenType::Semicolon => "ponto e vírgula".to_string(),
            TokenType::Colon => "dois pontos".to_string(),
            TokenType::Comma => "vírgula".to_string(),
            TokenType::String(..) => "string".to_string(),
            TokenType::Int(n) => format!("inteiro {}", n),
            TokenType::Newline => "quebra de linha".to_string(),
        }, self.span)
}
}

impl HasSpan for Token {
    fn span(&self) -> &Span {
        &self.span
    }
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
    fn read_number(&mut self) -> Option<Span> {
        let start = self.reader.offset();
        let mut end = self.reader.offset();
        let line = self.reader.line();
        let col = self.reader.col();
        while let Some((len, &c)) = self.reader.peek()
            && c.is_ascii_alphanumeric()
        {
            self.reader.next();
            end += len;
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
    fn escape_char(&mut self) -> Option<(usize, char)> {
        let (len, c) = self.reader.next()?;
        let size = len;
        match c {
            't' => Some('\t'),
            'n' => Some('\n'),
            'r' => Some('\r'),
            'f' => Some('\x0C'),
            'b' => Some('\x08'),
            'a' => Some('\x07'),
            's' => Some(' '),
            '\'' => Some('\''),
            '"' => Some('"'),
            _ => None,
        }
        .map(|c| (size, c))
    }
    fn read_string(&mut self) -> Option<(usize, String)> {
        let mut size = '"'.len_utf8();
        let mut content = String::new();
        while let Some((len, &c)) = self.reader.peek()
            && c != '"'
        {
            self.reader.next();
            size += len;
            if c == '\\' {
                let escaped = self.escape_char()?;
                content.push(escaped.1);
                size += escaped.0;
            } else {
                content.push(c);
            }
        }
        let (len, _) = self.reader.next()?;
        size += len;
        Some((size, content))
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
                    None
                }
                '\\' => {
                    self.reader.next();
                    let start2 = self.reader.offset();
                    let line2 = self.reader.line();
                    let col2 = self.reader.col();
                    if let Some((len2, &c2)) = self.reader.peek() {
                        if c2 == '\n' {
                            None
                        } else {
                            return Some(Err(TokenizerError {
                                span: Span {
                                    start: start2,
                                    end: start2 + len2,
                                    line: line2,
                                    col: col2,
                                },
                                error_type: TokenizerErrorType::UnexpectedCharacter,
                            }));
                        }
                    } else {
                        return Some(Err(TokenizerError {
                            span: Span {
                                start,
                                end: start + len,
                                line,
                                col,
                            },
                            error_type: TokenizerErrorType::UnexpectedCharacter,
                        }));
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
                _ if comment => None,
                _ if c.is_whitespace() => None,
                '.' => {
                    self.reader.next();
                    if let Some(span) = self.read_identifier() {
                        Some(Token {
                            token_type: TokenType::Directive,
                            span: Span {
                                start,
                                end: span.end,
                                line,
                                col,
                            },
                        })
                    } else {
                        return Some(Err(TokenizerError {
                            error_type: TokenizerErrorType::InvalidDirective,
                            span: Span {
                                start,
                                end: self.reader.offset(),
                                line,
                                col,
                            },
                        }));
                    }
                }
                '\'' => {
                    self.reader.next();
                    let n = self.reader.next().and_then(|(_, c)| {
                        if c == '\n' {
                            return None;
                        }
                        let n = if c == '\\' {
                            let (_, c2) = self.escape_char()?;
                            c2
                        } else {
                            c
                        };
                        let (_, c) = self.reader.next()?;
                        if c != '\'' { None } else { Some(n as isize) }
                    });
                    let span = Span {
                        start,
                        end: self.reader.offset(),
                        line,
                        col,
                    };
                    if let Some(n) = n {
                        Some(Token {
                            token_type: TokenType::Int(n),
                            span,
                        })
                    } else {
                        return Some(Err(TokenizerError {
                            error_type: TokenizerErrorType::UnendedChar,
                            span,
                        }));
                    }
                }
                '"' => {
                    self.reader.next();
                    let s = self.read_string();
                    let span = Span {
                        start,
                        end: self.reader.offset(),
                        line,
                        col,
                    };
                    if let Some((_, content)) = s {
                        Some(Token {
                            token_type: TokenType::String(content),
                            span,
                        })
                    } else {
                        return Some(Err(TokenizerError {
                            error_type: TokenizerErrorType::UnendedString,
                            span,
                        }));
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
                    Some(Token {
                        token_type: TokenType::Colon,
                        span: Span {
                            start,
                            end: self.reader.offset(),
                            line,
                            col,
                        },
                    })
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
                '-' | '0'..='9' => {
                    let negative = if c == '-' {
                        self.reader.next();
                        true
                    } else {
                        false
                    };
                    if let Some(mut span) = self.read_number() {
                        let s = self.source_map.get_span(&span);
                        let radix = s.get(0..2).and_then(|prefix| match prefix {
                            "0x" => Some(16),
                            "0b" => Some(2),
                            _ => None,
                        });
                        if radix.is_some() {
                            span.start += 2;
                        }
                        let s2 = self.source_map.get_span(&span);
                        let span = Span {
                            start,
                            end: span.end,
                            line,
                            col,
                        };
                        let Ok(n) = (match radix {
                            Some(base) => isize::from_str_radix(s2, base),
                            _ => s2.parse::<isize>(),
                        }) else {
                            return Some(Err(TokenizerError {
                                span,
                                error_type: TokenizerErrorType::InvalidNumber,
                            }));
                        };
                        Some(Token {
                            token_type: TokenType::Int(if negative { -n } else { n }),
                            span,
                        })
                    } else {
                        return Some(Err(TokenizerError {
                            span: Span {
                                start,
                                end: start + len,
                                line,
                                col,
                            },
                            error_type: TokenizerErrorType::InvalidNumber,
                        }));
                    }
                }
                _ if is_identifier_start(&c) => self.read_identifier().map(|span| Token {
                    token_type: TokenType::Identifier,
                    span,
                }),
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
            } else {
                self.reader.next();
            }
        }
        None
    }
}

#[derive(Error, Clone, Debug, PartialEq, Eq)]
#[error("Erro de sintaxe: {error_type} em {span:?}")]
pub struct TokenizerError {
    pub span: Span,
    #[source]
    pub error_type: TokenizerErrorType,
}

#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum TokenizerErrorType {
    #[error("Número inválido")]
    InvalidNumber,
    #[error("Diretiva inválida")]
    InvalidDirective,
    #[error("Caracter inexperado")]
    UnexpectedCharacter,
    #[error("String não terminada")]
    UnendedString,
    #[error("Caractere não terminado")]
    UnendedChar,
}

fn is_identifier_start(c: &char) -> bool {
    c.is_alphabetic()
}
fn is_identifier_body(c: &char) -> bool {
    c.is_alphanumeric() || *c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn lexer_assert(lexer: &mut Tokenizer) -> impl FnMut(TokenType, &str) {
        move |typ, content| {
            let token = lexer.next().unwrap().unwrap();
            assert_eq!(token.token_type, typ);
            assert_eq!(lexer.source_map.get_span(&token.span), content);
        }
    }

    #[test]
    fn test_tokens() {
        let source_map = SourceMap::from_content("5 .data
                TESTE1: .word 1, -2\\\n, 0xff, 0b11111111
                TESTE2: .asciz \"String\n ascii com \\n caracteres de\\tcontrole\"
                TESTE3: .byte ' ', '\\n'
            .text
            MAIN: LODD TESTE1; LOCO -1");
        let mut lexer = Tokenizer::new(&source_map);
        let mut assert_next = lexer_assert(&mut lexer);
        assert_next(TokenType::Int(5), "5");
        assert_next(TokenType::Directive, ".data");
        assert_next(TokenType::Newline, "\n");
        assert_next(TokenType::Identifier, "TESTE1");
        assert_next(TokenType::Colon, ":");
        assert_next(TokenType::Directive, ".word");
        assert_next(TokenType::Int(1), "1");
        assert_next(TokenType::Comma, ",");
        assert_next(TokenType::Int(-2), "-2");
        assert_next(TokenType::Comma, ",");
        assert_next(TokenType::Int(255), "0xff");
        assert_next(TokenType::Comma, ",");
        assert_next(TokenType::Int(255), "0b11111111");
        assert_next(TokenType::Newline, "\n");
        assert_next(TokenType::Identifier, "TESTE2");
        assert_next(TokenType::Colon, ":");
        assert_next(TokenType::Directive, ".asciz");
        assert_next(
            TokenType::String(String::from(
                "String\n ascii com \n caracteres de\tcontrole",
            )),
            "\"String\n ascii com \\n caracteres de\\tcontrole\"",
        );
        assert_next(TokenType::Newline, "\n");
        assert_next(TokenType::Identifier, "TESTE3");
        assert_next(TokenType::Colon, ":");
        assert_next(TokenType::Directive, ".byte");
        assert_next(TokenType::Int(' ' as isize), "' '");
        assert_next(TokenType::Comma, ",");
        assert_next(TokenType::Int('\n' as isize), "'\\n'");
        assert_next(TokenType::Newline, "\n");
        assert_next(TokenType::Directive, ".text");
        assert_next(TokenType::Newline, "\n");
        assert_next(TokenType::Identifier, "MAIN");
        assert_next(TokenType::Colon, ":");
        assert_next(TokenType::Identifier, "LODD");
        assert_next(TokenType::Identifier, "TESTE1");
        assert_next(TokenType::Semicolon, ";");
        assert_next(TokenType::Identifier, "LOCO");
        assert_next(TokenType::Int(-1), "-1");
    }

    fn assert_lexer_err(source_map_content: &'static str, typ: TokenizerErrorType, content: &str) {
        let source_map = SourceMap::from_content(source_map_content);
        let err = Tokenizer::new(&source_map)
            .collect::<Result<Vec<Token>, TokenizerError>>()
            .unwrap_err();
        assert_eq!(err.error_type, typ);
        assert_eq!(source_map.get_span(&err.span), content);
    }

    #[test]
    fn test_errors() {
        assert_lexer_err(". data", TokenizerErrorType::InvalidDirective, ".");
        assert_lexer_err("-abc", TokenizerErrorType::InvalidNumber, "-abc");
        assert_lexer_err("*.data", TokenizerErrorType::UnexpectedCharacter, "*");
        assert_lexer_err("\"*.data", TokenizerErrorType::UnendedString, "\"*.data");
        assert_lexer_err("'abcde", TokenizerErrorType::UnendedChar, "'ab");
        assert_lexer_err("'\n'", TokenizerErrorType::UnendedChar, "'\n");
        assert_lexer_err("'a '", TokenizerErrorType::UnendedChar, "'a ");
    }
}
