mod errors;
mod mir_builder;
mod regex;

use crate::{
    architecture::signals::ControlSignals,
    parsers::mal::{mir_builder::ControlSignalsBuilder, regex::parse_line},
};
use std::collections::HashMap;

pub use crate::parsers::mal::errors::{ParsingError, ParsingErrorType};

#[derive(Debug, Clone)]
pub struct MicroinstructionBuilder<'a> {
    pub lineno: usize,
    pub content: &'a str,
    pub mir: ControlSignalsBuilder<'a>,
}

#[derive(Debug, Clone)]
pub struct Microinstruction {
    pub content: String,
    pub mir: ControlSignals,
}

impl<'a> From<MicroinstructionBuilder<'a>> for Microinstruction {
    fn from(value: MicroinstructionBuilder<'a>) -> Self {
        Microinstruction {
            content: String::from(value.content),
            mir: value.mir.build(),
        }
    }
}

pub struct MALParser {}

impl MALParser {
    pub fn new() -> Self {
        MALParser {}
    }

    fn map_instructions<'a>(
        &self,
        source: &'a str,
    ) -> Result<(Vec<MicroinstructionBuilder<'a>>, HashMap<&'a str, usize>), ParsingError<'a>> {
        let mut instructions = Vec::new();
        let mut symbol_table = HashMap::new();
        for (lineno, content) in source.split('\n').enumerate() {
            match parse_line(content) {
                Some(Ok((name, mir))) => {
                    let mic = MicroinstructionBuilder {
                        lineno,
                        content,
                        mir,
                    };
                    instructions.push(mic);
                    symbol_table.insert(name, instructions.len() - 1);
                    // O limite da quantidade de instruções é 8 bits
                    if instructions.len() > 0xff {
                        return Err(ParsingError {
                            lineno,
                            content,
                            error_type: ParsingErrorType::MicroinstructionOverflow,
                        });
                    }
                }
                Some(Err(err)) => {
                    return Err(ParsingError {
                        lineno,
                        content,
                        error_type: err,
                    });
                }
                None => continue,
            };
        }
        Ok((instructions, symbol_table))
    }

    fn insert_addresses<'a>(
        &self,
        instructions: &mut Vec<MicroinstructionBuilder<'a>>,
        symbol_table: HashMap<&'a str, usize>,
    ) -> Result<(), ParsingError<'a>> {
        for mic in instructions.iter_mut() {
            if let Some(symb) = mic.mir.get_addr_symbol() {
                if let Some(addr) = symbol_table.get(symb) {
                    mic.mir.set_addr(*addr as u16);
                } else {
                    return Err(ParsingError {
                        lineno: mic.lineno,
                        content: mic.content,
                        error_type: ParsingErrorType::UnrecognizedSymbol(symb),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn parse_instructions<'a>(
        &self,
        source: &'a str,
    ) -> Result<Vec<Microinstruction>, ParsingError<'a>> {
        let (mut instructions, symbol_table) = self.map_instructions(source)?;
        self.insert_addresses(&mut instructions, symbol_table)?;
        Ok(instructions
            .into_iter()
            .map(|l| l.into())
            .collect::<Vec<Microinstruction>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;

    #[test]
    fn test_line_parse() {
        let (name, mir) = parse_line("0: pc := pc + 1; mar := pc; rd;")
            .unwrap()
            .unwrap();
        assert_eq!(name, "0");
        assert_eq!(mir.get_bool("amux"), Some(false));
        assert_eq!(mir.get_int("cond"), Some(0));
        assert_eq!(mir.get_int("alu"), Some(0));
        assert_eq!(mir.get_int("sh"), Some(0));
        assert_eq!(mir.get_bool("rd"), Some(true));
        assert_eq!(mir.get_bool("wr"), Some(false));
        assert_eq!(mir.get_bool("mar"), Some(true));
        assert_eq!(mir.get_bool("mbr"), Some(false));
        assert_eq!(mir.get_bool("enc"), Some(true));
        assert_eq!(mir.get_int("a"), Some(6));
        assert_eq!(mir.get_int("b"), Some(0));
        assert_eq!(mir.get_int("c"), Some(0));
        assert_eq!(mir.get_addr_symbol(), None);

        let (name, mir) = parse_line("0: pc := 1 + pc; mar := pc; rd;")
            .unwrap()
            .unwrap();
        assert_eq!(name, "0");
        assert_eq!(mir.get_bool("amux"), Some(false));
        assert_eq!(mir.get_int("cond"), Some(0));
        assert_eq!(mir.get_int("alu"), Some(0));
        assert_eq!(mir.get_int("sh"), Some(0));
        assert_eq!(mir.get_bool("rd"), Some(true));
        assert_eq!(mir.get_bool("wr"), Some(false));
        assert_eq!(mir.get_bool("mar"), Some(true));
        assert_eq!(mir.get_bool("mbr"), Some(false));
        assert_eq!(mir.get_bool("enc"), Some(true));
        assert_eq!(mir.get_int("a"), Some(6));
        assert_eq!(mir.get_int("b"), Some(0));
        assert_eq!(mir.get_int("c"), Some(0));
        assert_eq!(mir.get_addr_symbol(), None);

        assert!(
            parse_line("0: pc := 1 + pc; mar := amask; rd;")
                .unwrap()
                .is_err()
        );

        let (name, mir) = parse_line("balangodango1: tir := lshift(ir + ir); if n then goto 19;")
            .unwrap()
            .unwrap();
        assert_eq!(name, "balangodango1");
        assert_eq!(mir.get_bool("amux"), Some(false));
        assert_eq!(mir.get_int("cond"), Some(1));
        assert_eq!(mir.get_int("alu"), Some(0));
        assert_eq!(mir.get_int("sh"), Some(1));
        assert_eq!(mir.get_bool("rd"), Some(false));
        assert_eq!(mir.get_bool("wr"), Some(false));
        assert_eq!(mir.get_bool("mar"), Some(false));
        assert_eq!(mir.get_bool("mbr"), Some(false));
        assert_eq!(mir.get_bool("enc"), Some(true));
        assert_eq!(mir.get_int("a"), Some(3));
        assert_eq!(mir.get_int("b"), Some(3));
        assert_eq!(mir.get_int("c"), Some(4));
        assert_eq!(mir.get_addr_symbol(), Some("19"));

        let (name, mir) = parse_line("34: mar := a; mbr := ac; wr; goto 10;")
            .unwrap()
            .unwrap();
        assert_eq!(name, "34");
        assert_eq!(mir.get_bool("amux"), Some(false));
        assert_eq!(mir.get_int("cond"), Some(3));
        assert_eq!(mir.get_int("alu"), Some(2));
        assert_eq!(mir.get_int("sh"), Some(0));
        assert_eq!(mir.get_bool("rd"), Some(false));
        assert_eq!(mir.get_bool("wr"), Some(true));
        assert_eq!(mir.get_bool("mar"), Some(true));
        assert_eq!(mir.get_bool("mbr"), Some(true));
        assert_eq!(mir.get_bool("enc"), Some(false));
        assert_eq!(mir.get_int("a"), Some(1));
        assert_eq!(mir.get_int("b"), Some(10));
        assert_eq!(mir.get_int("c"), Some(0));
        assert_eq!(mir.get_addr_symbol(), Some("10"));

        assert!(parse_line("# teste: pc := pc + 1;").is_none());
    }

    #[test]
    fn test_code_equivalence() {
        let mp = MALParser::new();
        let ml1: Vec<u64> = mp
            .parse_instructions(&read_to_string("/home/henrique/code/mac1/teste.mal").unwrap())
            .expect("")
            .into_iter()
            .map(|m| u64::from(m.mir))
            .collect();
        let ml2: Vec<u64> = mp
            .parse_instructions(&read_to_string("/home/henrique/code/mac1/malde.mal").unwrap())
            .expect("")
            .into_iter()
            .map(|m| u64::from(m.mir))
            .collect();
        for i in 0..ml1.len() {
            if ml1[i] != ml2[i] {
                println!("{}", i);
                println!("Expected: {:064b}", ml1[i]);
                println!("Got     : {:064b}", ml2[i]);
            }
        }
        assert_eq!(ml1, ml2);
    }
}
