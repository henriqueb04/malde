pub mod control;
pub mod datapath;
pub mod events;
pub mod memory;
pub mod signals;

use std::sync::{Arc, Mutex};

use control::{ControlUnit, MicroMem};
use datapath::Datapath;
use memory::Memory;

use crate::architecture::{datapath::{DataRegisters, RegisterBank}, events::EventHandler};

#[derive(Debug)]
pub struct Cpu {
    datapath: Datapath,
    control_unit: ControlUnit,
    memory: Arc<Mutex<Memory>>,
}

impl Cpu {
    pub fn new(memory: Arc<Mutex<Memory>>, micro_mem: Arc<Mutex<MicroMem>>) -> Self {
        Cpu {
            memory,
            control_unit: ControlUnit::new(micro_mem),
            datapath: Datapath::new(),
        }
    }

    pub fn advance_microinstruction(&mut self, events: &mut EventHandler) -> (usize, usize) {
        self.control_unit.load_signals();
        self.datapath.clock(&self.control_unit.signals, events);
        self.memory.lock().unwrap().clock(
            &self.control_unit.signals,
            &self.datapath.mar,
            &mut self.datapath.mbr,
            events,
        );

        let (mpc, prev_mpc) = self.control_unit.advance(&self.datapath.alu_sigs);
        (mpc, prev_mpc)
    }

    pub fn get_registers(&self) -> DataRegisters {
        self.datapath.registers()
    }
    pub fn set_register(&mut self, register: usize, value: u16) {
        self.datapath.registers_mut()[register] = value;
    }

    pub fn reset(&mut self) {
        self.datapath.reset();
        self.control_unit.mpc = 0;
    }
}
