use std::fmt::Display;

use pastey::paste;
use thiserror::Error;

use crate::{architecture::signals::ControlSignals, parsers::source_map::Span};

macro_rules! make_setters {
    (
        $vis:vis struct $name:ident $(<$lt:lifetime>)? {
            $( $field:ident : $ty:ty ),+ $(,)?
        }
    ) => {
        #[derive(Debug, Default, Clone)]
        $vis struct $name $(<$lt>)? {
            $(
                $field: Option<$ty>,
            )+
        }

        paste! {
            #[allow(dead_code)]
            impl $(<$lt>)? $name $(<$lt>)? {
                pub fn new() -> $name {
                    Self::default()
                }

                $(
                    pub fn $field<'a>(&mut self, value: $ty, span: &'a Span) -> Result<$ty, ValueConflict<'a, $ty>> {
                        if let Some(v) = &self.$field && *v != value {
                            Err(ValueConflict { name: stringify!($field), before: v.clone(), after: value, span })
                        } else {
                            self.$field = Some(value.clone());
                            Ok(value)
                        }
                    }
                )+
                $(
                    pub fn [<get_ $field>](&self) -> Option<&$ty> {
                        self.$field.as_ref()
                    }
                )+
                $(
                    pub fn [<$field _force>]<'a>(&mut self, value: $ty) {
                        self.$field = Some(value.clone());
                    }
                )+
            }
        }
    };
}

make_setters! {
    pub struct ControlSignalsBuilder {
        amux: bool,
        cond: u8,
        alu: u8,
        sh: u8,
        mbr: bool,
        mar: bool,
        rd: bool,
        wr: bool,
        enc: bool,
        c: u8,
        b: u8,
        a: u8,
        addr_name: Span,
        syscall: bool,
    }
}

impl ControlSignalsBuilder {
    pub fn build(self, addr: u16) -> ControlSignals {
        ControlSignals {
            amux: self.get_amux().cloned().unwrap_or(false),
            cond: self.get_cond().cloned().unwrap_or(0),
            alu: self.get_alu().cloned().unwrap_or(0),
            sh: self.get_sh().cloned().unwrap_or(0),
            mbr: self.get_mbr().cloned().unwrap_or(false),
            mar: self.get_mar().cloned().unwrap_or(false),
            rd: self.get_rd().cloned().unwrap_or(false),
            wr: self.get_wr().cloned().unwrap_or(false),
            enc: self.get_enc().cloned().unwrap_or(false),
            c: self.get_c().cloned().unwrap_or(0),
            b: self.get_b().cloned().unwrap_or(0),
            a: self.get_a().cloned().unwrap_or(0),
            addr,
            syscall: self.get_syscall().cloned().unwrap_or(false),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("registrador {name} (antes: {before}, depois: {after})")]
pub struct ValueConflict<'a, T>
where
    T: Display,
{
    pub name: &'static str,
    pub before: T,
    pub after: T,
    pub span: &'a Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockable_errors() {
        let mut s = ControlSignalsBuilder::new();
        let span = Span::default();
        assert_eq!(s.a(1, &span), Ok(1));
        assert_eq!(s.b(2, &span), Ok(2));
        assert_ne!(s.b(3, &span), Ok(2));
        assert_eq!(s.b(2, &span), Ok(2));
        assert_eq!(
            s.b(3, &span),
            Err(ValueConflict {
                name: "b",
                before: 2,
                after: 3,
                span: &span,
            })
        );
        assert_eq!(
            s.a(2, &span),
            Err(ValueConflict {
                name: "a",
                before: 1,
                after: 2,
                span: &span,
            })
        );
        assert_eq!(s.b(2, &span), Ok(2));
    }
}
