pub mod control;
pub mod datapath;
pub mod events;
pub mod memory;
pub mod signals;

use std::{cell::RefCell, rc::Rc};

use control::{ControlUnit, MicroMem};
use datapath::Datapath;
use memory::Memory;

use crate::architecture::events::EventHandler;

pub struct Cpu {
    datapath: Datapath,
    control_unit: ControlUnit,
    memory: Rc<RefCell<Memory>>,
}

impl Cpu {
    pub fn new(memory: Rc<RefCell<Memory>>, micro_mem: Rc<RefCell<MicroMem>>) -> Self {
        Cpu {
            memory,
            control_unit: ControlUnit::new(micro_mem),
            datapath: Datapath::new(),
        }
    }

    pub fn advance_microinstruction(&mut self, events: &mut EventHandler) -> (usize, usize) {
        self.control_unit.load_signals();
        self.datapath.clock(&self.control_unit.signals, events);
        self.memory.borrow_mut().clock(
            &self.control_unit.signals,
            &self.datapath.mar,
            &mut self.datapath.mbr,
            events,
        );
        let (mpc, prev_mpc) = self.control_unit.advance(&self.datapath.alu_sigs);
        (mpc, prev_mpc)
    }

    pub fn get_registers(&self) -> (u16, u16, &[u16; 16]) {
        (
            self.datapath.mar,
            self.datapath.mbr,
            self.datapath.get_registers(),
        )
    }

    pub fn reset(&mut self) {
        self.datapath.reset();
        self.control_unit.mpc = 0;
    }
}
