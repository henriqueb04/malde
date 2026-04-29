use crate::architecture::signals::{ALUSignals, ControlSignals};

pub struct Sequencer {
    pub microinstructions: Vec<u32>,
    pub len: usize,
}

impl Sequencer {
    pub fn new(microinstructions: Vec<u32>) -> Self {
        let len = microinstructions.len();
        Sequencer {
            microinstructions,
            len,
        }
    }

    #[rustfmt::skip]
    fn load_instruction(&self, mpc: &usize, mir: &mut ControlSignals) {
        *mir = ControlSignals::from(&self.microinstructions[*mpc]);
    }
}

pub struct ControlUnit {
    pub signals: ControlSignals,
    pub sequencer: Sequencer,
    pub mpc: usize,
}

impl ControlUnit {
    pub fn new(sequencer: Sequencer) -> Self {
        ControlUnit {
            signals: ControlSignals::default(),
            sequencer,
            mpc: 0,
        }
    }

    pub fn load_signals(&mut self) {
        self.sequencer.load_instruction(&self.mpc, &mut self.signals);
    }

    pub fn advance(&mut self, alu_sigs: &ALUSignals) -> (usize, usize) {
        let old_mpc = self.mpc;
        self.mpc = match self.signals.cond {
            1 => if alu_sigs.n { self.signals.addr as usize } else { self.mpc + 1 },
            2 => if alu_sigs.z { self.signals.addr as usize } else { self.mpc + 1 },
            3 => self.signals.addr as usize,
            _ => self.mpc + 1,
        };
        if self.mpc >= self.sequencer.len {
            println!("Microinstruction pc has gone out of bounds! Reseting to 0.");
        }
        (self.mpc, old_mpc)
    }
}
