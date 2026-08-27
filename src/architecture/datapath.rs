use std::{fmt::Display, str::FromStr};

use crate::architecture::{
    events::EventHandler,
    signals::{ALUSignals, ControlSignals},
};

pub type RegisterBank = [u16; 16];

#[derive(Debug)]
pub struct Datapath {
    bus_a: u16,
    bus_b: u16,
    bus_c: u16,
    registers: RegisterBank,
    alu_out: u16,
    alu_in_a: u16,
    pub mar: u16,
    pub mbr: u16,
    pub alu_sigs: ALUSignals,
}

impl Datapath {
    pub fn new() -> Self {
        Datapath {
            bus_a: 0,
            bus_b: 0,
            bus_c: 0,
            mar: 0,
            mbr: 0,
            registers: Register::DEFAULT_VALUES,
            alu_in_a: 0,
            alu_out: 0,
            alu_sigs: ALUSignals { z: false, n: false },
        }
    }

    #[inline]
    fn get_register(&self, register: u8) -> u16 {
        self.registers[register as usize]
    }

    fn load_to_bus_a(&mut self, register: u8) {
        self.bus_a = self.get_register(register);
    }

    fn load_to_bus_b(&mut self, register: u8) {
        self.bus_b = self.get_register(register);
    }

    fn alu_add(&mut self) {
        (self.alu_out, _) = self.alu_in_a.overflowing_add(self.bus_b);
    }

    fn alu_and(&mut self) {
        self.alu_out = self.alu_in_a & self.bus_b;
    }

    fn alu_transparency(&mut self) {
        self.alu_out = self.alu_in_a;
    }

    fn alu_not(&mut self) {
        self.alu_out = self.alu_in_a ^ (-1i16 as u16);
    }

    fn alu_operate(&mut self, op: u8) {
        match op {
            0 => self.alu_add(),
            1 => self.alu_and(),
            2 => self.alu_transparency(),
            3 => self.alu_not(),
            _ => (),
        }
        self.alu_sigs.z = self.alu_out == 0;
        self.alu_sigs.n = self.alu_out & (1 << 15) != 0;
    }

    fn shift(&mut self, op: u8) {
        self.bus_c = match op {
            1 => self.alu_out << 1,
            2 => self.alu_out >> 1,
            _ => self.alu_out,
        }
    }

    fn load_to_register(&mut self, register: u8) {
        if !(5..=9).contains(&register) {
            self.registers[register as usize] = self.bus_c;
        }
    }

    fn load_to_mar(&mut self) {
        // Only load 12 bits into mar
        self.mar = self.bus_b & ((1 << 12) - 1) as u16;
    }

    fn load_to_mbr(&mut self) {
        self.mbr = self.bus_c;
    }

    pub fn clock(&mut self, signals: &ControlSignals, events: &mut EventHandler) {
        self.load_to_bus_a(signals.a);
        self.load_to_bus_b(signals.b);
        if signals.mar {
            let before = self.mar;
            self.load_to_mar();
            events.mar_write(before, self.mar);
        }
        self.alu_in_a = if signals.amux { self.mbr } else { self.bus_a };
        self.alu_operate(signals.alu);
        self.shift(signals.sh);
        if signals.mbr {
            let before = self.mbr;
            self.load_to_mbr();
            events.mbr_write(before, self.mbr);
        }
        if signals.enc {
            let before = self.get_register(signals.c);
            self.load_to_register(signals.c);
            events.register_write(signals.c, before, self.get_register(signals.c));
        }
    }

    pub fn reset(&mut self) {
        self.registers = Register::DEFAULT_VALUES;
        self.mar = 0;
        self.mbr = 0;
    }

    pub fn registers(&self) -> DataRegisters {
        DataRegisters(self.mar, self.mbr, self.registers)
    }
    pub fn registers_mut(&mut self) -> &mut RegisterBank {
        &mut self.registers
    }
}

#[derive(Debug, Clone)]
pub struct DataRegisters(pub u16, pub u16, pub RegisterBank);

#[allow(dead_code)]
impl DataRegisters {
    pub fn pc(&self) -> u16 {
        self.2[0]
    }
    pub fn ac(&self) -> u16 {
        self.2[1]
    }
    pub fn sp(&self) -> u16 {
        self.2[2]
    }
    pub fn ir(&self) -> u16 {
        self.2[3]
    }
    pub fn tir(&self) -> u16 {
        self.2[4]
    }
    pub fn zero(&self) -> u16 {
        self.2[5]
    }
    pub fn one(&self) -> u16 {
        self.2[6]
    }
    pub fn minus_one(&self) -> u16 {
        self.2[7]
    }
    pub fn amask(&self) -> u16 {
        self.2[8]
    }
    pub fn smask(&self) -> u16 {
        self.2[9]
    }
    pub fn a(&self) -> u16 {
        self.2[10]
    }
    pub fn b(&self) -> u16 {
        self.2[11]
    }
    pub fn c(&self) -> u16 {
        self.2[12]
    }
    pub fn d(&self) -> u16 {
        self.2[13]
    }
    pub fn e(&self) -> u16 {
        self.2[14]
    }
    pub fn f(&self) -> u16 {
        self.2[15]
    }
}

impl Default for DataRegisters {
    fn default() -> Self {
        DataRegisters(0, 0, Register::DEFAULT_VALUES)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Register {
    Pc,
    Ac,
    Sp,
    Ir,
    Tir,
    Zero,
    One,
    MinusOne,
    Amask,
    Smask,
    A,
    B,
    C,
    D,
    E,
    F,
    Alu,
    Mar,
    Mbr,
}

impl Register {
    pub const DEFAULT_VALUES: RegisterBank = [
        0,
        0,
        (1 << 12) as u16, // sp (no final da memória)
        0,
        0,
        0,                      // 0
        1,                      // 1
        -1i16 as u16,           // -1
        ((1 << 12) - 1) as u16, // amask
        ((1 << 8) - 1) as u16,  // smask
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    pub const NAMES: [&str; 16] = [
        "pc", "ac", "sp", "ir", "tir", "0", "1", "(-1)", "amask", "smask", "a", "b", "c", "d", "e",
        "f",
    ];
    pub const fn index(&self) -> Option<usize> {
        match self {
            Self::Pc => Some(0),
            Self::Ac => Some(1),
            Self::Sp => Some(2),
            Self::Ir => Some(3),
            Self::Tir => Some(4),
            Self::Zero => Some(5),
            Self::One => Some(6),
            Self::MinusOne => Some(7),
            Self::Amask => Some(8),
            Self::Smask => Some(9),
            Self::A => Some(10),
            Self::B => Some(11),
            Self::C => Some(12),
            Self::D => Some(13),
            Self::E => Some(14),
            Self::F => Some(15),
            _ => None,
        }
    }
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Pc => "pc",
            Self::Ac => "ac",
            Self::Sp => "sp",
            Self::Ir => "ir",
            Self::Tir => "tir",
            Self::Zero => "0",
            Self::One => "1",
            Self::MinusOne => "(-1)",
            Self::Amask => "amask",
            Self::Smask => "smask",
            Self::A => "a",
            Self::B => "b",
            Self::C => "c",
            Self::D => "d",
            Self::E => "e",
            Self::F => "f",
            Self::Alu => "(nenhum)",
            Self::Mar => "mar",
            Self::Mbr => "mbr",
        }
    }
}

impl FromStr for Register {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pc" => Ok(Self::Pc),
            "ac" => Ok(Self::Ac),
            "sp" => Ok(Self::Sp),
            "ir" => Ok(Self::Ir),
            "tir" => Ok(Self::Tir),
            "0" => Ok(Self::Zero),
            "1" => Ok(Self::One),
            "(-1)" => Ok(Self::MinusOne),
            "amask" => Ok(Self::Amask),
            "smask" => Ok(Self::Smask),
            "a" => Ok(Self::A),
            "b" => Ok(Self::B),
            "c" => Ok(Self::C),
            "d" => Ok(Self::D),
            "e" => Ok(Self::E),
            "f" => Ok(Self::F),
            "alu" => Ok(Self::Alu),
            "mar" => Ok(Self::Mar),
            "mbr" => Ok(Self::Mbr),
            _ => Err(()),
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
