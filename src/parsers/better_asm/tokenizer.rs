use std::{iter::Peekable, str::CharIndices};

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
}

impl HasSpan for Token {
    fn span(&self) -> &Span {
        &self.span
    }
}

pub struct Tokenizer<'a> {
    source_map: SourceMap<'a>,
    chars: SourceReader<'a>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(source_map: &SourceMap<'a>) -> Self {
        Tokenizer {
            source_map: source_map.clone(),
            chars: source_map.reader(),
        }
    }

    fn read_identifier(&mut self) -> Option<Span> {
        let start = self.chars.offset();
        let mut end = self.chars.offset();
        let line = self.chars.line();
        let col = self.chars.col();
        while let Some((_, &c)) = self.chars.peek()
            && is_identifier_body(&c)
        {
            self.chars.next();
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
    fn read_digits(&mut self) -> Option<Span> {
        let start = self.chars.offset();
        let mut end = self.chars.offset();
        let line = self.chars.line();
        let col = self.chars.col();
        while let Some((_, &c)) = self.chars.peek()
            && (c.is_ascii_digit() || c == '-')
        {
            self.chars.next();
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
    fn escape_char(&mut self) -> Option<(usize, char)> {
        let (_, c) = self.chars.next()?;
        let size = c.len_utf8();
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
        while let Some((_, &c)) = self.chars.peek()
            && c != '"'
        {
            self.chars.next();
            size += c.len_utf8();
            if c == '\\' {
                let escaped = self.escape_char()?;
                content.push(escaped.1);
                size += escaped.0;
            } else {
                content.push(c);
            }
        }
        let (_, c) = self.chars.next()?;
        size += c.len_utf8();
        Some((size, content))
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token, TokenizerError>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut comment = false;
        while let Some((l, &c)) = self.chars.peek() {
            let start = self.chars.offset();
            let line = self.chars.line();
            let col = self.chars.col();
            let t: Option<Token> = match c {
                '#' => {
                    comment = true;
                    None
                }
                '\n' => {
                    comment = false;
                    None
                }
                _ if comment => None,
                _ if c.is_whitespace() => None,
                '.' => {
                    self.chars.next();
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
                                end: self.chars.offset(),
                                line,
                                col,
                            },
                        }));
                    }
                }
                '\'' => {
                    self.chars.next();
                    let n = self.chars.next().and_then(|(_, c)| {
                        if c == '\n' {
                            return None;
                        }
                        let n = if c == '\\' {
                            let (s, c2) = self.escape_char()?;
                            c2
                        } else {
                            c
                        };
                        let (_, c) = self.chars.next()?;
                        if c != '\'' {
                            None
                        } else {
                            Some(n as isize)
                        }
                    });
                    let span = Span {
                        start,
                        end: self.chars.offset(),
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
                    self.chars.next();
                    let s = self.read_string();
                    let span = Span {
                        start,
                        end: self.chars.offset(),
                        line,
                        col,
                    };
                    if let Some((size, content)) = s {
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
                    self.chars.next();
                    let span = Span {
                        start,
                        end: self.chars.offset(),
                        line,
                        col,
                    };
                    Some(Token {
                        token_type: TokenType::Semicolon,
                        span,
                    })
                }
                ':' => {
                    self.chars.next();
                    Some(Token {
                        token_type: TokenType::Colon,
                        span: Span {
                            start,
                            end: self.chars.offset(),
                            line,
                            col
                        },
                    })
                }
                ',' => {
                    self.chars.next();
                    Some(Token {
                        token_type: TokenType::Comma,
                        span: Span {
                            start,
                            end: self.chars.offset(),
                            line,
                            col
                        },
                    })
                }
                '-' | '0'..='9' => {
                    // TODO: support binary and hex representations
                    let span = self.read_digits()?;
                    if let Ok(n) = self.source_map.get_span(&span).parse::<isize>() {
                        Some(Token {
                            token_type: TokenType::Int(n),
                            span,
                        })
                    } else {
                        return Some(Err(TokenizerError {
                            span,
                            error_type: TokenizerErrorType::InvalidNumber,
                        }));
                    }
                }
                _ if is_identifier_start(&c) => self.read_identifier().map(|span| Token {
                    token_type: TokenType::Identifier,
                    span,
                }),
                _ => {
                    self.chars.next();
                    return Some(Err(TokenizerError {
                        span: Span {
                            start,
                            end: self.chars.offset(),
                            line,
                            col
                        },
                        error_type: TokenizerErrorType::UnexpectedCharacter,
                    }));
                }
            };
            if let Some(token) = t {
                return Some(Ok(token));
            } else {
                self.chars.next();
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
    c.is_alphanumeric()
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
        let source_map = SourceMap {
            filename: "",
            content: "5 .data
                TESTE1: .word 1, 2
                TESTE2: .asciz \"String\n ascii com \\n caracteres de\\tcontrole\"
                TESTE3: .byte ' ', '\\n'
            .text
            MAIN: LODD TESTE1; LOCO -1",
        };
        let mut lexer = Tokenizer::new(&source_map);
        let mut assert_next = lexer_assert(&mut lexer);
        assert_next(TokenType::Int(5), "5");
        assert_next(TokenType::Directive, ".data");
        assert_next(TokenType::Identifier, "TESTE1");
        assert_next(TokenType::Colon, ":");
        assert_next(TokenType::Directive, ".word");
        assert_next(TokenType::Int(1), "1");
        assert_next(TokenType::Comma, ",");
        assert_next(TokenType::Int(2), "2");
        assert_next(TokenType::Identifier, "TESTE2");
        assert_next(TokenType::Colon, ":");
        assert_next(TokenType::Directive, ".asciz");
        assert_next(
            TokenType::String(String::from(
                "String\n ascii com \n caracteres de\tcontrole",
            )),
            "\"String\n ascii com \\n caracteres de\\tcontrole\"",
        );
        assert_next(TokenType::Identifier, "TESTE3");
        assert_next(TokenType::Colon, ":");
        assert_next(TokenType::Directive, ".byte");
        assert_next(TokenType::Int(' ' as isize), "' '");
        assert_next(TokenType::Comma, ",");
        assert_next(TokenType::Int('\n' as isize), "'\\n'");
        assert_next(TokenType::Directive, ".text");
        assert_next(TokenType::Identifier, "MAIN");
        assert_next(TokenType::Colon, ":");
        assert_next(TokenType::Identifier, "LODD");
        assert_next(TokenType::Identifier, "TESTE1");
        assert_next(TokenType::Semicolon, ";");
        assert_next(TokenType::Identifier, "LOCO");
        assert_next(TokenType::Int(-1), "-1");
    }

    fn assert_lexer_err(source_map_content: &'static str, typ: TokenizerErrorType, content: &str) {
        let source_map = SourceMap {
            filename: "",
            content: source_map_content,
        };
        let err = Tokenizer::new(&source_map)
            .collect::<Result<Vec<Token>, TokenizerError>>()
            .unwrap_err();
        assert_eq!(err.error_type, typ);
        assert_eq!(source_map.get_span(&err.span), content);
    }

    #[test]
    fn test_errors() {
        assert_lexer_err(". data", TokenizerErrorType::InvalidDirective, ".");
        assert_lexer_err("-abc", TokenizerErrorType::InvalidNumber, "-");
        assert_lexer_err("*.data", TokenizerErrorType::UnexpectedCharacter, "*");
        assert_lexer_err("\"*.data", TokenizerErrorType::UnendedString, "\"*.data");
        assert_lexer_err("'abcde", TokenizerErrorType::UnendedChar, "'ab");
        assert_lexer_err("'\n'", TokenizerErrorType::UnendedChar, "'\n");
        assert_lexer_err("'a '", TokenizerErrorType::UnendedChar, "'a ");
    }
}
