use crate::parsers::{
    lockable::ControlSignalsLockable,
    mal_regex::{ParsingError, parse_line},
};
use std::collections::HashMap;

pub struct MALParser<'a> {
    source: &'a str,
    symbol_table: HashMap<&'a str, usize>,
    instructions: Vec<ControlSignalsLockable<'a>>,
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
        for line in self.source.split('\n') {
            match parse_line(line) {
                Some(Ok((name, mir))) => {
                    // Faz com que os próximos valores no sequenciador mantenham os dados do anterior,
                    // modificando apenas as informações diferentes
                    // alguns valores, porém, são atribuídos com um valor padrão durante o parsing da linha
                    if let Some(previous) = self.instructions.last() {
                        self.instructions.push(previous.increment_self(&mir));
                    } else {
                        self.instructions.push(mir);
                    }
                    self.symbol_table.insert(name, self.instructions.len() - 1);
                    // O limite da quantidade de instruções é 8 bits
                    if self.instructions.len() > 0xff {
                        return Err(ParsingError::MicroinstructionOverflow);
                    }
                }
                Some(Err(err)) => return Err(err),
                None => continue,
            };
        }
        Ok(())
    }

    pub fn insert_addresses(&mut self) -> Result<(), ParsingError<'a>> {
        for mir in self.instructions.iter_mut() {
            if let Some(symb) = mir.get_addr_symbol() {
                if let Some(addr) = self.symbol_table.get(symb) {
                    let _ = mir.set_addr_force(*addr as u8);
                } else {
                    return Err(ParsingError::UnrecognizedSymbol(symb));
                }
            }
        }
        Ok(())
    }

    pub fn parse_instructions(
        &mut self,
    ) -> Result<&Vec<ControlSignalsLockable<'a>>, ParsingError<'a>> {
        self.map_instructions()?;
        self.insert_addresses()?;
        Ok(&self.instructions)
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
