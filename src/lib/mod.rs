mod control;
mod datapath;
mod signals;

use control::{ControlUnit, Sequencer};
use datapath::Datapath;

pub struct Cpu {
    datapath: Datapath,
    control_unit: ControlUnit,
}

impl Cpu {
    fn new(microinstructions: Box<[u32]>) -> Self {
        Cpu {
            datapath: Datapath::new(),
            control_unit: ControlUnit::new(Sequencer::new(microinstructions)),
        }
    }

    fn advance_microinstruction(&mut self) -> usize {
        // TODO: read and write to memory
        self.control_unit.load_signals();
        self.datapath.clock(&self.control_unit.signals);
        self.control_unit.advance(&self.datapath.alu_sigs)
    }

    fn advance_macroinstruction(&mut self) {
        let mut mpc = 1; // temporary value
        while mpc != 0 {
            mpc = self.advance_microinstruction();
        }
    }
}
