use log::warn;
use std::sync::{Arc, Mutex};

use crate::architecture::signals::{ALUSignals, ControlSignals};

const MICROMEM_MAX_SIZE: usize = 1 << 10;

#[derive(Debug, Default)]
pub struct MicroMem {
    pub microinstructions: Vec<u64>,
    pub len: usize,
}

impl MicroMem {
    pub fn new(mut microinstructions: Vec<u64>) -> Self {
        let len = microinstructions.len();
        if len > MICROMEM_MAX_SIZE {
            warn!("Tamanho excedido para memória de microinstrução! Descartando excedente");
            microinstructions.truncate(MICROMEM_MAX_SIZE);
        }
        MicroMem {
            microinstructions,
            len,
        }
    }

    fn get_instruction(&self, mpc: &usize) -> ControlSignals {
        if *mpc < self.len {
            ControlSignals::from(&self.microinstructions[*mpc])
        } else if self.len > 0 {
            ControlSignals::from(&self.microinstructions[0])
        } else {
            ControlSignals::default()
        }
    }
}

#[derive(Debug)]
pub struct ControlUnit {
    pub signals: ControlSignals,
    pub micro_mem: Arc<Mutex<MicroMem>>,
    pub prev_mpc: usize,
    pub mpc: usize,
}

impl ControlUnit {
    pub fn new(micro_mem: Arc<Mutex<MicroMem>>) -> Self {
        ControlUnit {
            signals: ControlSignals::default(),
            micro_mem,
            prev_mpc: 0,
            mpc: 0,
        }
    }

    pub fn load_signals(&mut self) {
        self.signals = self.micro_mem.lock().unwrap().get_instruction(&self.mpc);
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
        if self.mpc >= self.micro_mem.lock().unwrap().len {
            self.mpc = 0;
            warn!("MPC é maior que o tamanho da memória de microinstruções. Redefinindo como 0...");
        }
        self.prev_mpc = old_mpc;
        (self.mpc, old_mpc)
    }
}
