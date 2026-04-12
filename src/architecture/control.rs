use crate::architecture::signals::{ALUSignals, ControlSignals};

#[inline]
fn slice_bits(bits: &u32, start: usize, end: usize) -> u8 {
    let len = start - end;
    if len > 8 {
        println!("Trying to slice more than 8 bits from microinstruction.");
    }
    let mask = (1 << len) - 1;
    ((bits >> (32 - end)) & mask) as u8
}

fn get_bit(bits: &u32, i: u8) -> bool {
    ((bits >> (32 - i)) & 1) == 1
}

pub struct Sequencer {
    microinstructions: Box<[u32]>,
    len: usize,
}

impl Sequencer {
    pub fn new(microinstructions: Box<[u32]>) -> Self {
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
    sequencer: Sequencer,
    mpc: usize,
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

    pub fn advance(&mut self, alu_sigs: &ALUSignals) -> usize {
        self.mpc = match self.signals.cond {
            1 => if alu_sigs.n { self.signals.addr as usize } else { self.mpc + 1 },
            2 => if alu_sigs.z { self.signals.addr as usize } else { self.mpc + 1 },
            3 => self.signals.addr as usize,
            _ => self.mpc + 1,
        };
        if self.mpc >= self.sequencer.len {
            println!("Microinstruction pc has gone out of bounds! Reseting to 0.");
        }
        self.mpc
    }
}
