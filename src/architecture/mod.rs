pub mod control;
pub mod datapath;
pub mod memory;
pub mod signals;

use control::{ControlUnit, Sequencer};
use datapath::Datapath;
use memory::Memory;

pub struct Cpu {
    datapath: Datapath,
    control_unit: ControlUnit,
    memory: Memory,
}

impl Cpu {
    fn new(microinstructions: Box<[u32]>) -> Self {
        Cpu {
            memory: Memory::new(),
            control_unit: ControlUnit::new(Sequencer::new(microinstructions)),
            datapath: Datapath::new(),
        }
    }

    fn advance_microinstruction(&mut self) -> usize {
        self.control_unit.load_signals();
        self.datapath.clock(&self.control_unit.signals);
        self.memory.clock(
            &self.control_unit.signals,
            &self.datapath.mar,
            &mut self.datapath.mbr,
        );
        self.control_unit.advance(&self.datapath.alu_sigs)
    }

    fn advance_macroinstruction(&mut self) {
        let mut mpc = 1; // temporary value
        while mpc != 0 {
            mpc = self.advance_microinstruction();
        }
    }
}
