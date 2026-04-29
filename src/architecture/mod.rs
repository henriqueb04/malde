pub mod control;
pub mod datapath;
pub mod memory;
pub mod signals;

use control::{ControlUnit, Sequencer};
use datapath::Datapath;
use memory::Memory;

use crate::architecture::{memory::MEMORY_SIZE, signals::ControlSignals};

pub struct Cpu {
    datapath: Datapath,
    control_unit: ControlUnit,
    memory: Memory,
    initial_memory: Option<Vec<u16>>,
}

impl Cpu {
    pub fn new(microinstructions: Vec<u32>) -> Self {
        Cpu {
            memory: Memory::new(),
            control_unit: ControlUnit::new(Sequencer::new(microinstructions)),
            datapath: Datapath::new(),
            initial_memory: None,
        }
    }

    pub fn advance_microinstruction(&mut self) -> (usize, usize) {
        self.control_unit.load_signals();
        self.datapath.clock(&self.control_unit.signals);
        self.memory.clock(
            &self.control_unit.signals,
            &self.datapath.mar,
            &mut self.datapath.mbr,
        );
        self.control_unit.advance(&self.datapath.alu_sigs)
    }

    pub fn advance_macroinstruction(&mut self) {
        let mut mpc = 1; // temporary value
        while mpc != 0 {
            (mpc, _) = self.advance_microinstruction();
        }
    }

    pub fn zero_out_memory(&mut self) {
        self.memory.clear();
    }
    pub fn load_into_memory(&mut self, data: &Vec<u16>) -> bool {
        if data.len() > MEMORY_SIZE.into() {
            return false;
        }
        self.memory.load(0, data);
        true
    }

    pub fn init_memory(&mut self, data: Vec<u16>) {
        self.initial_memory = Some(data);
    }
    pub fn reset(&mut self) {
        self.zero_out_memory();
        self.datapath.reset();
        if let Some(mem) = self.initial_memory.take() {
            self.load_into_memory(&mem);
            self.initial_memory = Some(mem);
        }
        self.control_unit.mpc = 0;
    }

    pub fn is_ready(&self) -> bool {
        self.control_unit.sequencer.len > 0
    }

    pub fn load_microinstructions(&mut self, microinstructions: Vec<u32>) {
        self.control_unit.sequencer = Sequencer::new(microinstructions);
    }

    pub fn get_registors(&self) -> (u16, u16, &[u16; 16]) {
        (self.datapath.mar, self.datapath.mbr, self.datapath.get_registors())
    }

    pub fn get_control_signals(&self) -> &ControlSignals {
        &self.control_unit.signals
    }

    pub fn get_memory(&self) -> &[u16] {
        self.memory.get_ref()
    }
}
