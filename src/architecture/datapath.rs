use crate::architecture::{
    events::EventHandler,
    signals::{ALUSignals, ControlSignals},
};

pub type RegisterBank = [u16; 16];

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
            registers: Registers::DEFAULT_VALUES,
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
        self.registers = Registers::DEFAULT_VALUES;
        self.mar = 0;
        self.mbr = 0;
    }

    pub fn registers(&self) -> &RegisterBank {
        &self.registers
    }
    pub fn registers_mut(&mut self) -> &mut RegisterBank {
        &mut self.registers
    }
}

pub struct Registers;
impl Registers {
    pub const PC: usize = 0;
    pub const AC: usize = 1;
    pub const SP: usize = 2;
    pub const IR: usize = 3;
    pub const TIR: usize = 4;
    pub const ZERO: usize = 5;
    pub const ONE: usize = 6;
    pub const MINUS_ONE: usize = 7;
    pub const AMASK: usize = 8;
    pub const SMASK: usize = 9;
    pub const A: usize = 10;
    pub const B: usize = 11;
    pub const C: usize = 12;
    pub const D: usize = 13;
    pub const E: usize = 14;
    pub const F: usize = 15;
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
        "pc", "ac", "sp", "ir", "tir", "0", "1", "-1", "amask", "smask", "a", "b", "c", "d", "e",
        "f",
    ];
    pub fn get_index(register_name: &str) -> Option<u8> {
        match register_name {
            "pc" => Some(0),
            "ac" => Some(1),
            "sp" => Some(2),
            "ir" => Some(3),
            "tir" => Some(4),
            "0" => Some(5),
            "1" => Some(6),
            "(-1)" => Some(7),
            "amask" => Some(8),
            "smask" => Some(9),
            "a" => Some(10),
            "b" => Some(11),
            "c" => Some(12),
            "d" => Some(13),
            "e" => Some(14),
            "f" => Some(15),
            _ => None,
        }
    }
}
