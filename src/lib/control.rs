use crate::lib::signals::{ALUSignals, ControlSignals};

struct Sequencer {
    microinstructions: Box<[u32]>,
    len: usize,
}

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

impl Sequencer {
    fn new(microinstructions: Box<[u32]>) -> Self {
        let len = microinstructions.len();
        Sequencer {
            microinstructions,
            len,
        }
    }

    #[rustfmt::skip]
    fn load_instruction(&self, mpc: &usize, mir: &mut ControlSignals) {
        let microinstruction = &self.microinstructions[*mpc];
        mir.amux = get_bit(microinstruction, 0);
        mir.cond = slice_bits(microinstruction, 1, 3);
        mir.alu  = slice_bits(microinstruction, 3, 5);
        mir.sh   = slice_bits(microinstruction, 5, 7);
        mir.mbr  = get_bit(microinstruction, 7);
        mir.mar  = get_bit(microinstruction, 8);
        mir.rd   = get_bit(microinstruction, 9);
        mir.wr   = get_bit(microinstruction, 10);
        mir.enc  = get_bit(microinstruction, 11);
        mir.c    = slice_bits(microinstruction, 12, 16);
        mir.b    = slice_bits(microinstruction, 16, 20);
        mir.a    = slice_bits(microinstruction, 20, 24);
        mir.addr = slice_bits(microinstruction, 24, 32);
    }
}

pub struct ControlUnit {
    pub signals: ControlSignals,
    sequencer: Sequencer,
    mpc: usize,
}

impl ControlUnit {
    fn new(sequencer: Sequencer) -> Self {
        ControlUnit {
            signals: ControlSignals::default(),
            sequencer,
            mpc: 0,
        }
    }

    fn load_signals(&mut self) {
        self.sequencer.load_instruction(&self.mpc, &mut self.signals);
    }

    fn advance(&mut self, alu_sigs: &ALUSignals) -> usize {
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
