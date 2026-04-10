use crate::lib::signals::{ALUSignals, ControlSignals};

fn get_registor_index(registor_name: &str) -> Option<u8> {
    match registor_name {
        "pc" => Some(0),
        "ac" => Some(1),
        "sp" => Some(2),
        "ir" => Some(3),
        "tir" => Some(4),
        "0" => Some(5),
        "1" => Some(6),
        "-1" => Some(7),
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

const REGISTOR_NAMES: [&'static str; 16] = [
    "pc", "ac", "sp", "ir", "tir", "0", "1", "-1", "amask", "smask", "a", "b", "c", "d", "e", "f",
];
fn get_registor_name(registor_index: u8) -> &'static str {
    REGISTOR_NAMES[registor_index as usize]
}

pub struct Datapath {
    bus_a: u16,
    bus_b: u16,
    bus_c: u16,
    mar: u16,
    mbr: u16,
    registors: [u16; 16],
    alu_out: u16,
    alu_in_a: u16,
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
            registors: [
                0,
                0,
                0,
                0,
                0,
                0,                      // 0
                1,                      // 1
                ((1 << 16) - 1) as u16, // -1
                ((1 << 12) - 1) as u16, // amask
                ((1 << 8) - 1) as u16,  // smask
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            alu_in_a: 0,
            alu_out: 0,
            alu_sigs: ALUSignals { z: false, n: false },
        }
    }

    #[inline]
    fn get_registor(&self, registor: u8) -> u16 {
        self.registors[registor as usize]
    }

    fn load_to_bus_a(&mut self, registor: u8) {
        self.bus_a = self.get_registor(registor);
    }

    fn load_to_bus_b(&mut self, registor: u8) {
        self.bus_b = self.get_registor(registor);
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
        self.alu_out = self.alu_in_a ^ ((1 << 16 - 1) as u16);
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

    fn load_to_registor(&mut self, registor: u8) {
        match registor {
            5..=9 => println!(
                "Trying to set constant registor {}. That is probably a mistake. Ignoring...",
                get_registor_name(registor)
            ),
            _ => self.registors[registor as usize] = self.bus_c,
        }
    }

    fn load_to_mar(&mut self) {
        // Only load 12 bits into mar
        self.mar = self.bus_b & ((1 << 12) - 1) as u16;
    }

    fn load_to_mbr(&mut self) {
        self.mbr = self.bus_c;
    }

    pub fn clock(&mut self, signals: &ControlSignals) {
        self.load_to_bus_a(signals.a);
        self.load_to_bus_b(signals.b);
        if signals.mar {
            self.load_to_mar();
        }
        self.alu_in_a = if signals.amux { 1 } else { self.bus_a };
        self.alu_operate(signals.alu);
        self.shift(signals.sh);
        if signals.enc {
            self.load_to_registor(signals.c);
        }
        // TODO: Request read and write from memory
    }
}
