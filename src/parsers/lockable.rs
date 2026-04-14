use crate::architecture::signals::{
    CONTROL_SIGNAL_NAMES_B, CONTROL_SIGNAL_NAMES_U, ControlSignals,
};
use core::fmt;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ControlSignalsLockable<'a> {
    int_map: HashMap<&'static str, Option<u8>>,
    bool_map: HashMap<&'static str, Option<bool>>,
    addr_symbol: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueAlreadySet<T> {
    name: &'static str,
    before: T,
    now: T,
}

impl<T> fmt::Display for ValueAlreadySet<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "conflicting values {} and {} for signal {}",
            self.before, self.now, self.name
        )
    }
}

impl<'a> ControlSignalsLockable<'a> {
    pub fn new() -> Self {
        let mut int_map = HashMap::new();
        let mut bool_map = HashMap::new();
        for name in CONTROL_SIGNAL_NAMES_B {
            bool_map.insert(name, None);
        }
        for name in CONTROL_SIGNAL_NAMES_U {
            int_map.insert(name, None);
        }
        ControlSignalsLockable {
            int_map,
            bool_map,
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

    pub fn set_bool(&mut self, name: &'static str, v: bool) -> Result<bool, ValueAlreadySet<bool>> {
        if !CONTROL_SIGNAL_NAMES_B.contains(&name) {
            panic!("Tentativa de setar sinal inexistente \"{}\"!", name);
        }
        if let Some(a) = self.bool_map.get(name).unwrap()
            && *a != v
        {
            return Err(ValueAlreadySet {
                name,
                before: *a,
                now: v,
            });
        }
        self.bool_map.insert(name, Some(v));
        Ok(v)
    }

    pub fn set_int(&mut self, name: &'static str, v: u8) -> Result<u8, ValueAlreadySet<u8>> {
        if !CONTROL_SIGNAL_NAMES_U.contains(&name) {
            panic!("Tentativa de setar sinal inexistente \"{}\"!", name);
        }
        if let Some(a) = self.int_map.get(name).unwrap()
            && *a != v
        {
            return Err(ValueAlreadySet {
                name,
                before: *a,
                now: v,
            });
        }
        self.int_map.insert(name, Some(v));
        Ok(v)
    }

    pub fn set_addr_symbol(
        &mut self,
        symbol: &'a str,
    ) -> Result<&'a str, ValueAlreadySet<&'a str>> {
        if let Some(a) = self.addr_symbol {
            return Err(ValueAlreadySet {
                name: "addr",
                before: a,
                now: symbol,
            });
        }
        self.addr_symbol = Some(symbol);
        Ok(symbol)
    }

    pub fn set_addr_force(&mut self, value: u8) {
        self.int_map.insert("addr", Some(value));
    }

    pub fn swap_a_b(&mut self) {
        let a = *self.int_map.get("a").unwrap();
        self.int_map.insert("a", *self.int_map.get("b").unwrap());
        self.int_map.insert("b", a);
    }

    pub fn is_all_some(&self) -> bool {
        for name in CONTROL_SIGNAL_NAMES_B {
            if self.bool_map.get(name).unwrap().is_none() {
                return false;
            }
        }
        for name in CONTROL_SIGNAL_NAMES_U {
            if self.int_map.get(name).unwrap().is_none() {
                return false;
            }
        }
        return true;
    }

    pub fn is_all_none(&self) -> bool {
        for name in CONTROL_SIGNAL_NAMES_B {
            if self.bool_map.get(name).unwrap().is_some() {
                return false;
            }
        }
        for name in CONTROL_SIGNAL_NAMES_U {
            if self.int_map.get(name).unwrap().is_some() {
                return false;
            }
        }
        return true;
    }

    pub fn increment_self(&self, other: &ControlSignalsLockable<'a>) -> ControlSignalsLockable<'a> {
        let mut new = other.clone();
        for name in CONTROL_SIGNAL_NAMES_B {
            let _ = new.set_bool(name, self.get_bool(name).unwrap_or(false));
        }
        for name in CONTROL_SIGNAL_NAMES_U {
            let _ = new.set_int(name, self.get_int(name).unwrap_or(0));
        }
        new
    }
}

impl Into<ControlSignals> for ControlSignalsLockable<'_> {
    fn into(self) -> ControlSignals {
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
            addr: self.get_int("addr").unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockable_errors() {
        let mut s = ControlSignalsLockable::new();
        assert_eq!(s.set_int("a", 1), Ok(1));
        assert_eq!(s.set_int("b", 2), Ok(2));
        assert_ne!(s.set_int("b", 3), Ok(2));
        assert_eq!(s.set_int("b", 2), Ok(2));
        assert_eq!(
            s.set_int("b", 3),
            Err(ValueAlreadySet {
                name: "b",
                before: 2,
                now: 3
            })
        );
        assert_eq!(
            s.set_int("a", 2),
            Err(ValueAlreadySet {
                name: "a",
                before: 1,
                now: 2
            })
        );
        assert_eq!(s.set_int("b", 2), Ok(2));
    }
}
