use std::{cell::RefCell, rc::Rc};

use crate::architecture::signals::{ALUSignals, ControlSignals};

const MICROMEM_MAX_SIZE: usize = (1 << 10) - 1;

#[derive(Default)]
pub struct MicroMem {
    pub microinstructions: Vec<u64>,
    pub len: usize,
}

impl MicroMem {
    pub fn new(mut microinstructions: Vec<u64>) -> Self {
        let len = microinstructions.len();
        if len > MICROMEM_MAX_SIZE {
            println!("Tamanho excedido para memória de microinstrução! Descartando excedente");
            microinstructions.truncate(MICROMEM_MAX_SIZE);
        }
        MicroMem {
            microinstructions,
            len,
        }
    }

    fn load_instruction(&self, mpc: &usize, mir: &mut ControlSignals) {
        *mir = ControlSignals::from(&self.microinstructions[*mpc]);
    }
}

pub struct ControlUnit {
    pub signals: ControlSignals,
    pub micro_mem: Rc<RefCell<MicroMem>>,
    pub prev_mpc: usize,
    pub mpc: usize,
}

impl ControlUnit {
    pub fn new(micro_mem: Rc<RefCell<MicroMem>>) -> Self {
        ControlUnit {
            signals: ControlSignals::default(),
            micro_mem,
            prev_mpc: 0,
            mpc: 0,
        }
    }

    pub fn load_signals(&mut self) {
        self.micro_mem
            .borrow_mut()
            .load_instruction(&self.mpc, &mut self.signals);
    }

    pub fn advance(&mut self, alu_sigs: &ALUSignals) -> (usize, usize) {
        let old_mpc = self.mpc;
        self.mpc = match self.signals.cond {
            1 => {
                if alu_sigs.n {
                    self.signals.addr as usize
                } else {
                    self.mpc + 1
                }
            }
            2 => {
                if alu_sigs.z {
                    self.signals.addr as usize
                } else {
                    self.mpc + 1
                }
            }
            3 => self.signals.addr as usize,
            _ => self.mpc + 1,
        };
        if self.mpc >= self.micro_mem.borrow().len {
            println!("Microinstruction pc has gone out of bounds! Reseting to 0.");
        }
        self.prev_mpc = old_mpc;
        (self.mpc, old_mpc)
    }
}
