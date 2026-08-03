use std::fmt::Display;
use std::{collections::HashMap, iter::Peekable};

use thiserror::Error;

use crate::architecture::signals::ControlSignals;
use crate::parsers::better_mal::{
    mir_builder::*,
    tokenizer::{Token, TokenType, Tokenizer, TokenizerError, TokenizerErrorType},
};
use crate::parsers::source_map::{SourceMap, Span};
use crate::virtual_machine::Register;

pub struct MALParser<'a> {
    source_map: &'a SourceMap,
    lexer: Peekable<Tokenizer<'a>>,
    mappings: HashMap<&'a str, usize>,
    mics: Vec<Microinstruction>,
    pre_mics: Vec<(ControlSignalsBuilder, usize)>,
}

impl<'a> MALParser<'a> {
    pub fn new(source_map: &'a SourceMap) -> Self {
        MALParser {
            source_map,
            lexer: Tokenizer::new(source_map).peekable(),
            mappings: HashMap::new(),
            mics: Vec::new(),
            pre_mics: Vec::new(),
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
    */
    pub fn parse(mut self) -> Result<Vec<Microinstruction>, MALParsingError> {
        self.burn_newlines()
            .map_err(|err| MALParsingError::new(self.source_map, err))?;
        while self.lexer.peek().is_some() {
            self.parse_clock()
                .map_err(|err| MALParsingError::new(self.source_map, err))?;
        }
        for (pre_mic, lineno) in self.pre_mics.into_iter() {
            if let Some(addr_span) = pre_mic.get_addr_name() {
                let addr_name = self.source_map.get_span(&addr_span);
                if let Some(addr) = self.mappings.get(addr_name) {
                    self.mics.push(Microinstruction {
                        content: self.source_map.get_line(lineno).to_string(),
                        mir: pre_mic.build(*addr as u16),
                    });
                } else {
                    return Err(MALParsingError::new(
                        self.source_map,
                        ParsingError {
                            span: addr_span.clone(),
                            error_type: ParsingErrorType::UnrecognizedLabel,
                        },
                    ));
                }
            } else {
                self.mics.push(Microinstruction {
                    content: self.source_map.get_line(lineno).to_string(),
                    mir: pre_mic.build(0),
                });
            }
        }
        Ok(self.mics)
    }

    fn parse_clock(&mut self) -> Result<(), ParsingError> {
        loop {
            let first = self.expect_identifier()?;
            let second = self.peek_kind()?;
            if let Some(sec) = second
                && *sec == TokenType::Colon
            {
                self.lexer.next();
                self.mappings
                    .insert(self.source_map.get(&first), self.pre_mics.len());
                continue;
            }
            let lineno = first.span.line;
            let mir = self.read_clock(first)?;
            self.expect_newline()?;
            self.pre_mics.push((mir, lineno));
            break;
        }
        Ok(())
    }

    fn read_clock(&mut self, first: Token) -> Result<ControlSignalsBuilder, ParsingError> {
        let mut first = first.mapped(self.source_map);
        let mut mir = ControlSignalsBuilder::new();
        loop {
            match &first.token_type {
                TokenType::Syscall => {
                    mir.syscall(true, &first.span)?;
                }
                TokenType::Wr => {
                    mir.wr(true, &first.span)?;
                }
                TokenType::Rd => {
                    mir.rd(true, &first.span)?;
                }
                TokenType::Goto | TokenType::Then => self.read_goto(&mut mir, first)?,
                TokenType::If => {
                    self.read_conditional(&mut mir, first)?;
                }
                TokenType::Register(..) => {
                    self.read_assingment(&mut mir, first)?;
                }
                _ => {
                    return Err(ParsingError {
                        span: first.span.clone(),
                        error_type: ParsingErrorType::UnexpectedToken(
                            "instrução".to_string(),
                            first,
                        ),
                    });
                }
            }
            self.expect(TokenType::Semicolon)?;
            if self.peek_kind()?.is_none_or(|t| *t == TokenType::Newline) {
                break;
            }
            first = self.next("")?;
        }
        Ok(mir)
    }

    fn read_goto(
        &mut self,
        mir: &mut ControlSignalsBuilder,
        first: Token,
    ) -> Result<(), ParsingError> {
        if first.token_type == TokenType::Then {
            self.lexer.next();
        }
        let id = self.next_unmapped("rótulo")?;
        if id.token_type != TokenType::Identifier {
            return Err(ParsingError {
                span: id.span.clone(),
                error_type: ParsingErrorType::UnexpectedToken("rótulo".to_string(), id),
            });
        }
        mir.addr_name(id.span.clone(), &id.span)?;
        // Se não já estiver definido, definir com 3
        let _ = mir.cond(3, &first.span);
        Ok(())
    }

    fn read_conditional(
        &mut self,
        mir: &mut ControlSignalsBuilder,
        first: Token,
    ) -> Result<(), ParsingError> {
        let condition = self.next("condição (n ou z)")?;
        match &condition.token_type {
            TokenType::Condition(cond) => {
                mir.cond(*cond, &first.span)?;
            }
            _ => {
                return Err(ParsingError {
                    span: first.span.clone(),
                    error_type: ParsingErrorType::UnexpectedToken(
                        "condição (n ou z)".to_string(),
                        first,
                    ),
                });
            }
        }
        let goto_first = self.next("then ou goto")?;
        self.read_goto(mir, goto_first)?;
        Ok(())
    }

    fn read_assingment(
        &mut self,
        mir: &mut ControlSignalsBuilder,
        first: Token,
    ) -> Result<(), ParsingError> {
        let assign = self.expect(TokenType::Assign)?;
        let op_first = self.next("operação")?;
        let OperationInfo {
            shift,
            alu_op,
            reg_a,
            reg_b,
        } = match &op_first.token_type {
            TokenType::Shift(..) => self.read_shift(op_first)?,
            TokenType::Register(..) | TokenType::AluFunc(..) => self.read_operation(op_first)?,
            _ => {
                return Err(ParsingError {
                    span: op_first.span.clone(),
                    error_type: ParsingErrorType::UnexpectedToken(
                        "operação da ula".to_string(),
                        op_first,
                    ),
                });
            }
        };
        let TokenType::Register(reg_dest) = first.token_type else {
            panic!("Registrador destino da operação não é um registrador!");
        };
        let dest_is_mar = reg_dest == Register::Mar;
        let dest_is_mbr = reg_dest == Register::Mbr;
        if let Some(dest) = reg_dest.index() {
            // Se o destino for do banco de registradores
            mir.c(dest as u8, &first.span)?;
            mir.enc(true, &first.span)?;
        } else if dest_is_mar {
            // Se o destino for MAR
            if alu_op.0 != 2 {
                // Se não for a operação de transparência
                return Err(ParsingError {
                    span: alu_op.1,
                    error_type: ParsingErrorType::IlegalOperation("mar"),
                });
            } else if let Some((_, span)) = &shift {
                // Se tentar usar lshift ou rshift
                return Err(ParsingError {
                    span: span.clone(),
                    error_type: ParsingErrorType::IlegalOperation("mar"),
                });
            } else if reg_a.0 == Register::Mbr {
                // Se tentar colocar MBR direto no MAR
                return Err(ParsingError {
                    span: reg_a.1,
                    error_type: ParsingErrorType::ImplossibleRoute("mbr", "mar"),
                });
            }
        } else if dest_is_mbr {
            mir.mbr(true, &first.span)?;
        }
        if let Some(mar) = reg_b
            .as_ref()
            .filter(|reg_b| reg_b.0 == Register::Mar)
            .or_else(|| (reg_a.0 == Register::Mar).then_some(&reg_a))
        {
            // Se tentar acessar o valor de MAR
            return Err(ParsingError {
                span: mar.1.clone(),
                error_type: ParsingErrorType::NotAcessibleForOperation("mar"),
            });
        } else if let Some(alu) = reg_b
            .as_ref()
            .filter(|reg_b| reg_b.0 == Register::Alu)
            .or_else(|| (reg_a.0 == Register::Alu).then_some(&reg_a))
        {
            // Se tentar usar ALU como operando
            return Err(ParsingError {
                span: alu.1.clone(),
                error_type: ParsingErrorType::NotARealRegister,
            });
        }
        if !dest_is_mar {
            // Se o destino não for MAR, definir os sinais da operação
            if let Some(shift) = shift {
                mir.sh(shift.0, &shift.1)?;
            } else {
                mir.sh(0, &assign.span)?;
            }
            mir.alu(alu_op.0, &alu_op.1)?;
        }
        // Se tentar usar MBR como registrador b ou se o destino for MAR, trocar
        let (reg_a, mut reg_b) = if dest_is_mar
            || (reg_b.as_ref().map(|b| &b.0) == Some(&Register::Mbr) && reg_a.0 != Register::Mbr)
        {
            (reg_b, Some(reg_a))
        } else {
            (Some(reg_a), reg_b)
        };
        if let Some(reg_a) = reg_a {
            match &reg_a.0 {
                Register::Mbr => {
                    mir.amux(true, &reg_a.1)?;
                }
                a => {
                    if let Some(a) = a.index().map(|a| a as u8) {
                        // Se é necessário trocar a definição de A e B
                        // Ex.: pc := a + b; mbr := b + a;
                        // É possível trocar quando:
                        // A operação tem dois operandos
                        // reg_a já está em B
                        // ou não há nada em A ou reg_b já está em A
                        if let Some(reg_b) = reg_b.as_mut()
                            && let Some(b) = reg_b.0.index().map(|b| b as u8)
                            && let Some(mir_b) = mir.get_b()
                            && a == *mir_b
                            && mir.get_a().is_none_or(|mir_a| *mir_a == b)
                        {
                            mir.a(b, &reg_b.1)?;
                            mir.b(a, &reg_a.1)?;
                            *reg_b = reg_a.clone();
                        } else {
                            mir.a(a, &reg_a.1)?;
                        }
                        mir.amux(false, &reg_a.1)?;
                    } else {
                        return Err(ParsingError {
                            span: reg_a.1,
                            error_type: ParsingErrorType::UnrecognizedRegister,
                        });
                    }
                }
            };
        }
        if let Some(reg_b) = reg_b {
            match reg_b.0 {
                b if b.index().is_some() => {
                    let b = b.index().unwrap() as u8;
                    if let Err(conflict) = mir.b(b, &reg_b.1) {
                        // Se é possível trocar A e B sem gerar conflitos
                        // Ex.: pc := pc + 1; mar := pc;
                        if let Some(&a) = mir.get_a()
                            && a == b
                            && mir.get_mar().is_none()
                        {
                            mir.b_force(a);
                            mir.a_force(conflict.before);
                            // Garantia de que não vai ser trocado mais de uma vez
                            mir.mar(dest_is_mar, &first.span)?;
                        } else {
                            return Err(conflict.into());
                        }
                    }
                }
                _ => {
                    return Err(ParsingError {
                        span: reg_b.1,
                        error_type: ParsingErrorType::UnrecognizedRegister,
                    });
                }
            };
        }
        // Se chegar até aqui e o destino for MAR, todos os erros já foram checados.
        // So está sendo definido aqui porque é verificado se foi definido antes de trocar A e B
        if dest_is_mar {
            mir.mar(true, &first.span)?;
        }
        Ok(())
    }

    fn read_shift(&mut self, first: Token) -> Result<OperationInfo, ParsingError> {
        if let TokenType::Shift(sh) = first.token_type {
            self.expect(TokenType::LeftParen)?;
            let op_first = self.next("operação da ula")?;
            let mut op_info = self.read_operation(op_first)?;
            self.expect(TokenType::RightParen)?;
            op_info.shift = Some((sh, first.span));
            Ok(op_info)
        } else {
            panic!("Tentativa de ler expressão de shift sem token de shift");
        }
    }

    fn read_operation(&mut self, first: Token) -> Result<OperationInfo, ParsingError> {
        match first.token_type {
            TokenType::Register(reg_a) => {
                if let Some(typ) = self.peek_kind()?
                    && *typ == TokenType::Plus
                {
                    // Soma
                    let plus = self.next("+")?;
                    let reg_b = self.expect_register()?;
                    Ok(OperationInfo {
                        shift: None,
                        alu_op: (0, plus.span),
                        reg_a: (reg_a, first.span),
                        reg_b: Some(reg_b),
                    })
                } else {
                    // Transparência
                    Ok(OperationInfo {
                        shift: None,
                        alu_op: (2, first.span.clone()),
                        reg_a: (reg_a, first.span),
                        reg_b: None,
                    })
                }
            }
            TokenType::AluFunc(1) => {
                // Bitwise And
                self.expect(TokenType::LeftParen)?;
                let reg_a = self.expect_register()?;
                self.expect(TokenType::Comma)?;
                let reg_b = self.expect_register()?;
                self.expect(TokenType::RightParen)?;
                Ok(OperationInfo {
                    shift: None,
                    alu_op: (1, first.span),
                    reg_a,
                    reg_b: Some(reg_b),
                })
            }
            TokenType::AluFunc(3) => {
                // Not
                self.expect(TokenType::LeftParen)?;
                let reg_a = self.expect_register()?;
                self.expect(TokenType::RightParen)?;
                Ok(OperationInfo {
                    shift: None,
                    alu_op: (3, first.span),
                    reg_a,
                    reg_b: None,
                })
            }
            _ => Err(ParsingError {
                span: first.span.clone(),
                error_type: ParsingErrorType::UnexpectedToken("operação da ula".to_string(), first),
            }),
        }
    }

    fn next_unmapped(&mut self, msg: &str) -> Result<Token, ParsingError> {
        Ok(self.lexer.next().ok_or_else(|| ParsingError {
            span: self.source_map.end(),
            error_type: ParsingErrorType::UnexpectedEnd(msg.to_string()),
        })??)
    }
    fn next(&mut self, msg: &str) -> Result<Token, ParsingError> {
        let t = self.next_unmapped(msg)?;
        Ok(t.mapped(self.source_map))
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
    fn expect_identifier(&mut self) -> Result<Token, ParsingError> {
        let t = self.next_unmapped("rótulo ou instrução")?;
        if t.token_type != TokenType::Identifier {
            Err(ParsingError {
                span: t.span.clone(),
                error_type: ParsingErrorType::UnexpectedToken("rótulo ou instrução".to_string(), t),
            })
        } else {
            Ok(t)
        }
    }
    fn expect_register(&mut self) -> Result<(Register, Span), ParsingError> {
        let t = self.next("registrador")?;
        if let TokenType::Register(reg) = t.token_type {
            Ok((reg, t.span))
        } else {
            Err(ParsingError {
                span: t.span.clone(),
                error_type: ParsingErrorType::UnexpectedToken("registrador".to_string(), t),
            })
        }
    }
    fn expect_newline(&mut self) -> Result<(), ParsingError> {
        if self.lexer.peek().is_none() {
            return Ok(());
        }
        self.expect(TokenType::Newline)?;
        self.burn_newlines()?;
        Ok(())
    }
    fn expect(&mut self, typ: TokenType) -> Result<Token, ParsingError> {
        let t = self.next(&typ.to_string())?;
        if t.token_type != typ {
            return Err(ParsingError {
                span: t.span.clone(),
                error_type: ParsingErrorType::UnexpectedToken(typ.to_string(), t),
            });
        }
        Ok(t)
    }
    fn burn_newlines(&mut self) -> Result<(), ParsingError> {
        while let Some(typ) = self.peek_kind()?
            && *typ == TokenType::Newline
        {
            self.next_unmapped("quebra de linha")?;
        }
        Ok(())
    }
}

struct OperationInfo {
    shift: Option<(u8, Span)>,
    alu_op: (u8, Span),
    reg_a: (Register, Span),
    reg_b: Option<(Register, Span)>,
}

#[derive(Debug, PartialEq, Eq)]
struct ParsingError {
    pub span: Span,
    pub error_type: ParsingErrorType,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub struct MALParsingError {
    display: String,
    pub span: Span,
    pub error_type: ParsingErrorType,
}

impl MALParsingError {
    fn new(source_map: &SourceMap, parsing_error: ParsingError) -> Self {
        let display = format!(
            "Erro ao ler linha {}, coluna {}:\n{}\n\n{}",
            parsing_error.span.line,
            parsing_error.span.col,
            source_map.highlight_in_line(&parsing_error.span),
            parsing_error.error_type
        );
        MALParsingError {
            display,
            span: parsing_error.span,
            error_type: parsing_error.error_type,
        }
    }
}

impl Display for MALParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParsingErrorType {
    #[error(transparent)]
    TokenError(TokenizerErrorType),
    #[error("Conflito de valores: {0}")]
    ValueConflict(String),
    #[error("Experava {0}, mas não foi encontrado")]
    UnexpectedEnd(String),
    #[error("Esperava {0}, mas foi encontrado {1}")]
    UnexpectedToken(String, Token),
    #[error("alu não pode ser usado como registrador em operação da ula")]
    NotARealRegister,
    #[error("O registrador {0} não pode receber essa operação")]
    IlegalOperation(&'static str),
    #[error(
        "O registrador {0} não pode ser usado para atribuir valor para o registrador {1} diretamente"
    )]
    ImplossibleRoute(&'static str, &'static str),
    #[error("O registrador {0} não é acessível para operações da ula")]
    NotAcessibleForOperation(&'static str),
    #[error("Registrador não reconhecido")]
    UnrecognizedRegister,
    #[error("Rótulo não encontrado")]
    UnrecognizedLabel,
}

impl From<TokenizerError> for ParsingError {
    fn from(value: TokenizerError) -> Self {
        ParsingError {
            span: value.span,
            error_type: ParsingErrorType::TokenError(value.error_type),
        }
    }
}

impl<T: Display> From<ValueConflict<'_, T>> for ParsingError {
    fn from(value: ValueConflict<'_, T>) -> Self {
        ParsingError {
            span: value.span.clone(),
            error_type: ParsingErrorType::ValueConflict(value.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Microinstruction {
    pub content: String,
    pub mir: ControlSignals,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::parsers::mal::MALParser as OldParser;

    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn test_parser() {
        let source_map = SourceMap::from_content("0: pc := pc + 1; mar := pc; rd;");
        let source_map2 = SourceMap::from_content("0: mar := pc; pc := pc + 1; rd;");

        let lexer = Tokenizer::new(&source_map);
        let tokens = lexer.collect::<Result<Vec<_>, TokenizerError>>().unwrap();
        let pretty_tokens = tokens
            .into_iter()
            .map(|t| t.mapped(&source_map))
            .map(|t| (t.span.line, t.token_type))
            .collect::<Vec<_>>();
        println!("{:?}", pretty_tokens);

        let parser = MALParser::new(&source_map);
        let mics = parser.parse().unwrap();
        assert_eq!(
            mics[0].mir,
            ControlSignals {
                amux: false,
                cond: 0,
                alu: 0,
                sh: 0,
                rd: true,
                wr: false,
                mar: true,
                mbr: false,
                enc: true,
                a: 6,
                b: 0,
                c: 0,
                addr: 0,
                syscall: false,
            }
        );
        let parser2 = MALParser::new(&source_map2);
        assert_eq!(parser2.parse().unwrap()[0].mir, mics[0].mir);

        let source_map = SourceMap::from_content("0: pc := a + b; mbr := b + a;");
        let source_map2 = SourceMap::from_content("0: mbr := b + a; pc := a + b;");

        let parser = MALParser::new(&source_map);
        let expected = ControlSignals {
            mbr: true,
            enc: true,
            a: 10,
            b: 11,
            ..Default::default()
        };
        assert_eq!(parser.parse().unwrap()[0].mir, expected);
        let parser2 = MALParser::new(&source_map2);
        assert_eq!(
            parser2.parse().unwrap()[0].mir,
            ControlSignals {
                a: 11,
                b: 10,
                ..expected
            }
        );

        let source_map = SourceMap::from_content(
            "0: f := lshift(1 + (-1)); wr; rd; syscall; if n then goto 1;\n1: if z goto 2;\n2: goto 0;",
        );
        assert_eq!(
            MALParser::new(&source_map)
                .parse()
                .unwrap()
                .into_iter()
                .map(|m| m.mir)
                .collect::<Vec<_>>(),
            vec![
                ControlSignals {
                    sh: 1,
                    a: 6,
                    b: 7,
                    c: 15,
                    enc: true,
                    wr: true,
                    rd: true,
                    syscall: true,
                    cond: 1,
                    addr: 1,
                    ..Default::default()
                },
                ControlSignals {
                    cond: 2,
                    addr: 2,
                    ..Default::default()
                },
                ControlSignals {
                    cond: 3,
                    addr: 0,
                    ..Default::default()
                },
            ]
        )
    }

    // Foi observado que o parser antigo continha um bug que foi corrigido no parser novo.
    // Ex: mar := pc; pc := pc + 1
    // Em expressões como a acima, o parser antigo não conseguia fazer a inversão de operadores corretamente.
    // Parser novo:   a = 6, b = 2, c = 2, mar = 1
    // Parser antigo: a = 2, b = 2, c = 2, mar = 1
    // Logo, o teste abaixo foi invalidado.

    // #[test]
    // fn test_equivalence() {
    //     let source_map = SourceMap::from_filepath("./malde.mal").unwrap();
    //     let parser1 = MALParser::new(&source_map);
    //     let mics1 = parser1
    //         .parse()
    //         .unwrap()
    //         .into_iter()
    //         .map(|m| m.mir)
    //         .enumerate()
    //         .collect::<Vec<_>>();
    //     // Parser anterior que utilizava regex
    //     let parser2 = OldParser::new();
    //     let mics2 = parser2
    //         .parse_instructions(fs::read_to_string("./malde.mal").unwrap().as_str())
    //         .unwrap()
    //         .into_iter()
    //         .map(|m| m.mir)
    //         .enumerate()
    //         .collect::<Vec<_>>();
    //     assert_eq!(mics1, mics2);
    // }
}
