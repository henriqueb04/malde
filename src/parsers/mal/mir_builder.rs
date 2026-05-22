use crate::architecture::signals::{
    CONTROL_SIGNAL_NAMES_B, CONTROL_SIGNAL_NAMES_U, ControlSignals,
};
use crate::parsers::mal::errors::{ValueAlreadySet, ValueConflictType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ControlSignalsBuilder<'a> {
    int_map: HashMap<&'static str, Option<u8>>,
    bool_map: HashMap<&'static str, Option<bool>>,
    addr_int: u16,
    addr_symbol: Option<&'a str>,
}

impl<'a> ControlSignalsBuilder<'a> {
    pub fn new() -> Self {
        let mut int_map = HashMap::new();
        let mut bool_map = HashMap::new();
        for name in CONTROL_SIGNAL_NAMES_B {
            bool_map.insert(name, None);
        }
        for name in CONTROL_SIGNAL_NAMES_U {
            int_map.insert(name, None);
        }
        ControlSignalsBuilder {
            int_map,
            bool_map,
            addr_int: 0,
            addr_symbol: None,
        }
    }

    pub fn get_bool(&self, name: &'static str) -> Option<bool> {
        if !CONTROL_SIGNAL_NAMES_B.contains(&name) {
            panic!("Tentativa de verificar sinal inexistente \"{}\"", name);
        }
        *self.bool_map.get(name).unwrap()
    }

    pub fn get_int(&self, name: &'static str) -> Option<u8> {
        if !CONTROL_SIGNAL_NAMES_U.contains(&name) {
            panic!("Tentativa de verificar sinal inexistente \"{}\"", name);
        }
        *self.int_map.get(name).unwrap()
    }

    pub fn get_addr_symbol(&self) -> Option<&'a str> {
        self.addr_symbol
    }

    pub fn set_bool(&mut self, name: &'static str, v: bool) -> Result<bool, ValueAlreadySet<'a>> {
        if !CONTROL_SIGNAL_NAMES_B.contains(&name) {
            panic!("Tentativa de setar sinal bool inexistente \"{}\"!", name);
        }
        if let Some(a) = self.bool_map.get(name).unwrap()
            && *a != v
        {
            return Err(ValueAlreadySet {
                name,
                conflict: ValueConflictType::Bool {
                    before: *a,
                    after: v,
                },
            });
        }
        self.bool_map.insert(name, Some(v));
        Ok(v)
    }

    pub fn set_int(&mut self, name: &'static str, v: u8) -> Result<u8, ValueAlreadySet<'a>> {
        if !CONTROL_SIGNAL_NAMES_U.contains(&name) {
            panic!("Tentativa de setar sinal int inexistente \"{}\"!", name);
        }
        if let Some(a) = self.int_map.get(name).unwrap()
            && *a != v
        {
            return Err(ValueAlreadySet {
                name,
                conflict: ValueConflictType::Int {
                    before: *a,
                    after: v,
                },
            });
        }
        self.int_map.insert(name, Some(v));
        Ok(v)
    }

    pub fn set_int_force(&mut self, name: &'static str, v: u8) {
        self.int_map.insert(name, Some(v));
    }

    pub fn set_addr_symbol(&mut self, symbol: &'a str) -> Result<&'a str, ValueAlreadySet<'a>> {
        if let Some(a) = self.addr_symbol {
            return Err(ValueAlreadySet {
                name: "addr",
                conflict: ValueConflictType::Str {
                    before: a,
                    after: symbol,
                },
            });
        }
        self.addr_symbol = Some(symbol);
        Ok(symbol)
    }

    pub fn set_addr(&mut self, value: u16) {
        self.addr_int = value;
    }
    pub fn get_addr(&self) -> u16 {
        self.addr_int
    }

    pub fn swap_a_b(&mut self) {
        let a = *self.int_map.get("a").unwrap();
        self.int_map.insert("a", *self.int_map.get("b").unwrap());
        self.int_map.insert("b", a);
    }

    pub fn set_defaults(&mut self) {
        // Valores padrão, são ignorados se já estiverem presentes
        let _ = self.set_bool("amux", false);
        let _ = self.set_int("cond", 0);
        let _ = self.set_int("alu", 0);
        let _ = self.set_int("sh", 0);
        let _ = self.set_bool("mbr", false);
        let _ = self.set_bool("mar", false);
        let _ = self.set_bool("rd", false);
        let _ = self.set_bool("wr", false);
        let _ = self.set_bool("enc", false);
        let _ = self.set_int("c", 0);
        let _ = self.set_int("b", 0);
        let _ = self.set_int("a", 0);
    }

    pub fn build(self) -> ControlSignals {
        ControlSignals {
            amux: self.get_bool("amux").unwrap_or(false),
            cond: self.get_int("cond").unwrap_or(0),
            alu: self.get_int("alu").unwrap_or(0),
            sh: self.get_int("sh").unwrap_or(0),
            mbr: self.get_bool("mbr").unwrap_or(false),
            mar: self.get_bool("mar").unwrap_or(false),
            rd: self.get_bool("rd").unwrap_or(false),
            wr: self.get_bool("wr").unwrap_or(false),
            enc: self.get_bool("enc").unwrap_or(false),
            c: self.get_int("c").unwrap_or(0),
            b: self.get_int("b").unwrap_or(0),
            a: self.get_int("a").unwrap_or(0),
            addr: self.get_addr(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockable_errors() {
        let mut s = ControlSignalsBuilder::new();
        assert_eq!(s.set_int("a", 1), Ok(1));
        assert_eq!(s.set_int("b", 2), Ok(2));
        assert_ne!(s.set_int("b", 3), Ok(2));
        assert_eq!(s.set_int("b", 2), Ok(2));
        assert_eq!(
            s.set_int("b", 3),
            Err(ValueAlreadySet {
                name: "b",
                conflict: ValueConflictType::Int {
                    before: 2,
                    after: 3
                },
            })
        );
        assert_eq!(
            s.set_int("a", 2),
            Err(ValueAlreadySet {
                name: "a",
                conflict: ValueConflictType::Int {
                    before: 1,
                    after: 2
                },
            })
        );
        assert_eq!(s.set_int("b", 2), Ok(2));
    }
}
