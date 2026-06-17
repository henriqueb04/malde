use crate::architecture::datapath::RegisterBank;
use crate::architecture::events::MachineEvents;
use crate::architecture::memory::{DATA_SEGMENT_START, Memory, MemoryArray, TEXT_SEGMENT_START};
use std::cell::Ref;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    architecture::{Cpu, control::MicroMem},
    parsers::{
        mac::{ASMParser, DEFAULT_KEYWORDS, ParsingError as ASMParsingError},
        mal::{MALParser, Microinstruction, ParsingError as MALParsingError},
    },
};

pub use crate::architecture::{datapath::REGISTER_NAMES, memory::MEMORY_SIZE};

#[derive(Debug, Default)]
pub struct VMResponse {
    pub mpc: usize,
    pub prev_mpc: usize,
    pub last_events: MachineEvents,
}

#[derive(Default)]
pub enum VMState {
    Active,
    #[default]
    Halted,
}

pub struct VM {
    keywords: HashMap<String, String>,
    state: VMState,
    initial_memory: Option<(Vec<u16>, Vec<u16>)>,
    memory: Rc<RefCell<Memory>>,
    micro_mem: Rc<RefCell<MicroMem>>,
    cpu: Cpu,
    microinstructions: Vec<Microinstruction>,
    // cur_instruction: usize,
}

impl Default for VM {
    fn default() -> Self {
        VM::new()
    }
}

impl VM {
    pub fn new() -> Self {
        let memory = Rc::new(RefCell::new(Memory::new()));
        let micro_mem = Rc::new(RefCell::new(MicroMem::new(Vec::new())));
        VM {
            keywords: HashMap::from(
                DEFAULT_KEYWORDS.map(|(k, v)| (String::from(k), String::from(v))),
            ),
            memory: Rc::clone(&memory),
            micro_mem: Rc::clone(&micro_mem),
            cpu: Cpu::new(Rc::clone(&memory), Rc::clone(&micro_mem)),
            state: VMState::Halted,
            microinstructions: Vec::new(),
            initial_memory: None
        }
    }

    pub fn assemble_mic<'a>(&mut self, source: &'a str) -> Result<(), MALParsingError<'a>> {
        let parser = MALParser::new();
        let microinstructions = parser.parse_instructions(source)?;
        self.micro_mem.replace(MicroMem::new(
            microinstructions
                .iter()
                .map(|m| m.mir.clone().into())
                .collect(),
        ));
        self.state = VMState::Active;
        self.microinstructions = microinstructions;
        Ok(())
    }

    pub fn assemble_mac<'a>(&mut self, source: &'a str) -> Result<(), ASMParsingError<'a>> {
        let mut parser = ASMParser::new(&self.keywords);
        let mem = parser.parse_text(source)?;
        self.set_initial_memory(mem.0, mem.1);
        self.reset_memory();
        Ok(())
    }

    pub fn get_microinstructions(&self) -> &Vec<Microinstruction> {
        &self.microinstructions
    }

    pub fn is_ready(&self) -> bool {
        self.micro_mem.borrow().len > 0
    }

    // Memory
    pub fn set_initial_memory(&mut self, initial_instructions: Vec<u16>, initial_data: Vec<u16>) {
        self.initial_memory = Some((initial_instructions, initial_data));
    }
    pub fn reset_memory(&mut self) {
        if let Some((initial_instructions, initial_data)) = self.initial_memory.as_ref() {
            let mut memory = self.memory.borrow_mut();
            memory.clear();
            memory.load(TEXT_SEGMENT_START, initial_instructions);
            memory.load(DATA_SEGMENT_START - 1, &[0]); // HALT de segurança
            memory.load(DATA_SEGMENT_START, initial_data);
        }
    }
    pub fn get_memory(&self) -> Ref<'_, MemoryArray> {
        Ref::map(self.memory.borrow(), |memory| memory.get_ref())
    }

    // Cpu
    pub fn advance_microinstruction(&mut self) -> VMResponse {
        match &self.state {
            VMState::Active => {
                let (prev_mar, mar, last_events) = self.cpu.advance_microinstruction();
                VMResponse {
                    mpc: prev_mar,
                    prev_mpc: mar,
                    last_events,
                }
            }

            _ => Default::default(),
        }
    }
    pub fn advance_macroinstruction(&mut self) -> VMResponse {
        let mut res = VMResponse::default();
        loop {
            if let Some(slot) = res.last_events.memory_read_start.as_ref().map(|v| v.slot)
                && slot < DATA_SEGMENT_START
            {
                // self.cur_instruction = slot;
                break;
            }
            res = self.advance_microinstruction();
        }
        res
    }
    pub fn reset(&mut self) {
        self.state = VMState::Active;
        let mut memory = self.memory.borrow_mut();
        if let Some(mem) = self.initial_memory.take() {
            memory.clear();
            memory.load(TEXT_SEGMENT_START, &mem.0);
            memory.load(DATA_SEGMENT_START, &mem.1);
            self.initial_memory = Some(mem);
        }
        self.cpu.reset();
    }
    pub fn get_registers(&self) -> (u16, u16, &RegisterBank) {
        self.cpu.get_registers()
    }
}
