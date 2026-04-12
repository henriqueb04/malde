use core::fmt;

#[derive(Default)]
pub struct ControlSignalsLockable {
    pub amux: Option<bool>,
    pub cond: Option<u8>,
    pub alu: Option<u8>,
    pub sh: Option<u8>,
    pub mbr: Option<bool>,
    pub mar: Option<bool>,
    pub rd: Option<bool>,
    pub wr: Option<bool>,
    pub enc: Option<bool>,
    pub c: Option<u8>,
    pub b: Option<u8>,
    pub a: Option<u8>,
    pub addr: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueAlreadySet<T> {
    name: &'static str,
    before: T,
    now: T,
}

impl<T> fmt::Display for ValueAlreadySet<T>
    where T: fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "conflicting values {} and {} for signal {}", self.before, self.now, self.name)
    }
}

impl ControlSignalsLockable {
    fn is_all_none(&self) -> bool {
        self.amux.is_none() &&
        self.cond.is_none() &&
        self.alu.is_none() &&
        self.sh.is_none() &&
        self.mbr.is_none() &&
        self.mar.is_none() &&
        self.rd.is_none() &&
        self.wr.is_none() &&
        self.enc.is_none() &&
        self.c.is_none() &&
        self.b.is_none() &&
        self.a.is_none() &&
        self.addr.is_none()
    }

    pub fn set_amux(&mut self, value: bool) -> Result<bool, ValueAlreadySet<bool>> {
        if let Some(v) = self.amux && v != value {
            return Err(ValueAlreadySet { name: "amux", before: v, now: value });
        }
        self.amux = Some(value);
        Ok(value)
    }
    pub fn set_cond(&mut self, value: u8) -> Result<u8, ValueAlreadySet<u8>> {
        if let Some(v) = self.cond && v != value {
            return Err(ValueAlreadySet { name: "cond", before: v, now: value });
        }
        self.cond = Some(value);
        Ok(value)
    }
    pub fn set_alu(&mut self, value: u8) -> Result<u8, ValueAlreadySet<u8>> {
        if let Some(v) = self.alu && v != value {
            return Err(ValueAlreadySet { name: "alu", before: v, now: value });
        }
        self.alu = Some(value);
        Ok(value)
    }
    pub fn set_sh(&mut self, value: u8) -> Result<u8, ValueAlreadySet<u8>> {
        if let Some(v) = self.sh && v != value {
            return Err(ValueAlreadySet { name: "sh", before: v, now: value });
        }
        self.sh = Some(value);
        Ok(value)
    }
    pub fn set_mbr(&mut self, value: bool) -> Result<bool, ValueAlreadySet<bool>> {
        if let Some(v) = self.mbr && v != value {
            return Err(ValueAlreadySet { name: "mbr", before: v, now: value });
        }
        self.mbr = Some(value);
        Ok(value)
    }
    pub fn set_mar(&mut self, value: bool) -> Result<bool, ValueAlreadySet<bool>> {
        if let Some(v) = self.mar && v != value {
            return Err(ValueAlreadySet { name: "mar", before: v, now: value });
        }
        self.mar = Some(value);
        Ok(value)
    }
    pub fn set_rd(&mut self, value: bool) -> Result<bool, ValueAlreadySet<bool>> {
        if let Some(v) = self.rd && v != value {
            return Err(ValueAlreadySet { name: "rd", before: v, now: value });
        }
        self.rd = Some(value);
        Ok(value)
    }
    pub fn set_wr(&mut self, value: bool) -> Result<bool, ValueAlreadySet<bool>> {
        if let Some(v) = self.wr && v != value {
            return Err(ValueAlreadySet { name: "wr", before: v, now: value });
        }
        self.wr = Some(value);
        Ok(value)
    }
    pub fn set_enc(&mut self, value: bool) -> Result<bool, ValueAlreadySet<bool>> {
        if let Some(v) = self.enc && v != value {
            return Err(ValueAlreadySet { name: "enc", before: v, now: value });
        }
        self.enc = Some(value);
        Ok(value)
    }
    pub fn set_c(&mut self, value: u8) -> Result<u8, ValueAlreadySet<u8>> {
        if let Some(v) = self.c && v != value {
            return Err(ValueAlreadySet { name: "c", before: v, now: value });
        }
        self.c = Some(value);
        Ok(value)
    }
    pub fn set_b(&mut self, value: u8) -> Result<u8, ValueAlreadySet<u8>> {
        if let Some(v) = self.b && v != value {
            return Err(ValueAlreadySet { name: "b", before: v, now: value });
        }
        self.b = Some(value);
        Ok(value)
    }
    pub fn set_a(&mut self, value: u8) -> Result<u8, ValueAlreadySet<u8>> {
        if let Some(v) = self.a && v != value {
            return Err(ValueAlreadySet { name: "a", before: v, now: value });
        }
        self.a = Some(value);
        Ok(value)
    }
    pub fn set_addr(&mut self, value: u8) -> Result<u8, ValueAlreadySet<u8>> {
        if let Some(v) = self.addr && v != value {
            return Err(ValueAlreadySet { name: "addr", before: v, now: value });
        }
        self.addr = Some(value);
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockable_errors() {
        let mut s = ControlSignalsLockable::default();
        assert_eq!(s.set_a(1), Ok(1));
        assert_eq!(s.set_b(2), Ok(2));
        assert_ne!(s.set_b(3), Ok(2));
        assert_eq!(s.set_b(2), Ok(2));
        assert_eq!(s.set_b(3), Err(ValueAlreadySet { name: "b", before: 2, now: 3 }));
        assert_eq!(s.set_a(2), Err(ValueAlreadySet { name: "a", before: 1, now: 2 }));
        assert_eq!(s.set_b(2), Ok(2));
    }
}
