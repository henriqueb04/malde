use std::{iter::Peekable, str::CharIndices};

use thiserror::Error;

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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub lineno: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap<'a> {
    pub filename: &'a str,
    pub content: &'a str,
}

impl<'a> SourceMap<'a> {
    pub fn get_span(&self, span: &Span) -> &'a str {
        &self.content[span.start..span.end]
    }
    pub fn get(&self, token: &Token) -> &'a str {
        self.get_span(&token.span)
    }
    pub fn end(&self) -> Span {
        Span {
            start: self.content.len(),
            end: self.content.len(),
            lineno: self.content.lines().count(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
}

pub struct Tokenizer<'a> {
    source_map: SourceMap<'a>,
    chars: Peekable<CharIndices<'a>>,
    cur_line: usize,
    start: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(source_map: &SourceMap<'a>) -> Self {
        Tokenizer {
            source_map: source_map.clone(),
            chars: source_map.content.char_indices().peekable(),
            cur_line: 1,
            start: 0,
        }
    }

    fn read_identifier(&mut self) -> Option<Span> {
        let mut end = self.start;
        while let Some(&(_, c)) = self.chars.peek()
            && is_identifier_body(&c)
        {
            self.chars.next();
            end += c.len_utf8();
        }
        if self.start != end {
            let span = Span {
                start: self.start,
                end,
                lineno: self.cur_line,
            };
            self.start = end;
            Some(span)
        } else {
            None
        }
    }
    fn read_digits(&mut self) -> Option<Span> {
        let mut end = self.start;
        while let Some(&(_, c)) = self.chars.peek()
            && (c.is_ascii_digit() || c == '-')
        {
            self.chars.next();
            end += c.len_utf8();
        }
        if self.start != end {
            let span = Span {
                start: self.start,
                end,
                lineno: self.cur_line,
            };
            self.start = end;
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
        while let Some(&(_, c)) = self.chars.peek()
            && c != '"'
        {
            self.chars.next();
            size += c.len_utf8();
            if c == '\\' {
                let escaped = self.escape_char()?;
                content.push(escaped.1);
                size += escaped.0;
            } else {
                if c == '\n' {
                    self.cur_line += 1;
                }
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
        while let Some(&(i, c)) = self.chars.peek() {
            self.start = i;
            let t: Option<Token> = match c {
                '#' => {
                    comment = true;
                    None
                }
                '\n' => {
                    self.cur_line += 1;
                    comment = false;
                    None
                }
                _ if comment => None,
                _ if c.is_whitespace() => None,
                '.' => {
                    let st = self.start;
                    self.chars.next();
                    self.start += c.len_utf8();
                    if let Some(span) = self.read_identifier() {
                        Some(Token {
                            token_type: TokenType::Directive,
                            span: Span {
                                start: st,
                                end: span.end,
                                lineno: span.lineno,
                            },
                        })
                    } else {
                        return Some(Err(TokenizerError {
                            error_type: TokenizerErrorType::InvalidDirective,
                            span: Span {
                                start: st,
                                end: st + c.len_utf8(),
                                lineno: self.cur_line,
                            },
                        }));
                    }
                }
                '\'' => {
                    let start = self.start;
                    let mut size = 1;
                    self.chars.next();
                    let n = self.chars.next().and_then(|(_, c)| {
                        size += c.len_utf8();
                        if c == '\n' {
                            return None;
                        }
                        let n = if c == '\\' {
                            let (s, c2) = self.escape_char()?;
                            size += s;
                            c2
                        } else {
                            c
                        };
                        let (_, c) = self.chars.next()?;
                        if c != '\'' {
                            None
                        } else {
                            size += c.len_utf8();
                            Some(n as isize)
                        }
                    });
                    if let Some(n) = n {
                        Some(Token {
                            token_type: TokenType::Int(n),
                            span: Span {
                                start,
                                end: start + size,
                                lineno: self.cur_line,
                            },
                        })
                    } else {
                        return Some(Err(TokenizerError {
                            error_type: TokenizerErrorType::UnendedChar,
                            span: Span {
                                start,
                                end: start + c.len_utf8(),
                                lineno: self.cur_line,
                            },
                        }));
                    }
                }
                '"' => {
                    let start = self.start;
                    self.chars.next();
                    if let Some((size, content)) = self.read_string() {
                        Some(Token {
                            token_type: TokenType::String(content),
                            span: Span {
                                start,
                                end: start + size,
                                lineno: self.cur_line,
                            },
                        })
                    } else {
                        return Some(Err(TokenizerError {
                            error_type: TokenizerErrorType::UnendedString,
                            span: Span {
                                start,
                                end: start + c.len_utf8(),
                                lineno: self.cur_line,
                            },
                        }));
                    }
                }
                ';' => {
                    let span = Span {
                        start: self.start,
                        end: self.start + c.len_utf8(),
                        lineno: self.cur_line,
                    };
                    self.chars.next();
                    Some(Token {
                        token_type: TokenType::Semicolon,
                        span,
                    })
                }
                ':' => {
                    let span = Span {
                        start: self.start,
                        end: self.start + c.len_utf8(),
                        lineno: self.cur_line,
                    };
                    self.chars.next();
                    Some(Token {
                        token_type: TokenType::Colon,
                        span,
                    })
                }
                ',' => {
                    let span = Span {
                        start: self.start,
                        end: self.start + c.len_utf8(),
                        lineno: self.cur_line,
                    };
                    self.chars.next();
                    Some(Token {
                        token_type: TokenType::Comma,
                        span,
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
                    return Some(Err(TokenizerError {
                        span: Span {
                            start: self.start,
                            end: self.start + c.len_utf8(),
                            lineno: self.cur_line,
                        },
                        error_type: TokenizerErrorType::UnexpectedCharacter,
                    }));
                }
            };
            if let Some(token) = t {
                return Some(Ok(token));
            } else {
                self.start += c.len_utf8();
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
        assert_lexer_err("\"*.data", TokenizerErrorType::UnendedString, "\"");
        assert_lexer_err("'abcde", TokenizerErrorType::UnendedChar, "'");
        assert_lexer_err("'\n'", TokenizerErrorType::UnendedChar, "'");
        assert_lexer_err("'a '", TokenizerErrorType::UnendedChar, "'");
    }
}
