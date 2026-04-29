use crate::{
    architecture::signals::ControlSignals,
    parsers::{lockable::ControlSignalsLockable, mal_regex::parse_line},
};
use std::{collections::HashMap, fmt::Display};

pub use crate::parsers::mal_regex::ParsingError as ParsingErrorType;

pub struct ParsingError<'a> {
    lineno: usize,
    content: &'a str,
    error_type: ParsingErrorType<'a>,
}

impl Display for ParsingError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Erro na linha {}: {}\n    \"{}\"", self.lineno + 1, self.error_type, self.content)
    }
}

#[derive(Debug, Clone)]
pub struct Microinstruction<'a> {
    lineno: usize,
    content: &'a str,
    mir: ControlSignalsLockable<'a>,
}

pub struct MALParser<'a> {
    source: &'a str,
    symbol_table: HashMap<&'a str, usize>,
    instructions: Vec<Microinstruction<'a>>,
}

impl<'a> MALParser<'a> {
    pub fn new(source: &'a str) -> Self {
        MALParser {
            source,
            symbol_table: HashMap::new(),
            instructions: Vec::new(),
        }
    }

    pub fn map_instructions(&mut self) -> Result<(), ParsingError<'a>> {
        for (lineno, content) in self.source.split('\n').enumerate() {
            println!("{}: {}", lineno, content);
            match parse_line(content) {
                Some(Ok((name, mir))) => {
                    // Faz com que os próximos valores no sequenciador mantenham os dados do anterior,
                    // modificando apenas as informações diferentes
                    // alguns valores, porém, são atribuídos com um valor padrão durante o parsing da linha
                    let mic = if let Some(Microinstruction { mir: previous, .. }) =
                        self.instructions.last()
                    {
                        Microinstruction {
                            lineno,
                            content,
                            mir: previous.increment_self(&mir),
                        }
                    } else {
                        Microinstruction {
                            lineno,
                            content,
                            mir,
                        }
                    };
                    self.instructions.push(mic);
                    self.symbol_table.insert(name, self.instructions.len() - 1);
                    // O limite da quantidade de instruções é 8 bits
                    if self.instructions.len() > 0xff {
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
        Ok(())
    }

    pub fn insert_addresses(&mut self) -> Result<(), ParsingError<'a>> {
        for mic in self.instructions.iter_mut() {
            if let Some(symb) = mic.mir.get_addr_symbol() {
                if let Some(addr) = self.symbol_table.get(symb) {
                    let _ = mic.mir.set_addr_force(*addr as u8);
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

    pub fn parse_instructions(&mut self) -> Result<Vec<ControlSignals>, ParsingError<'a>> {
        self.map_instructions()?;
        self.insert_addresses()?;
        Ok(self
            .instructions
            .iter()
            .map(|l| ControlSignals::from(l.mir.clone()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
