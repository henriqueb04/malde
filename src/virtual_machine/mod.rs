use crate::architecture::datapath::RegistorBank;
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

pub use crate::architecture::{
    datapath::REGISTOR_NAMES, memory::MEMORY_SIZE, signals::ControlSignals,
};

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
}

impl VM {
    pub fn new() -> Self {
        let memory = Rc::new(RefCell::new(Memory::new()));
        let micro_mem = Rc::new(RefCell::new(MicroMem::new(Vec::new())));
        VM {
            keywords: HashMap::from(
                DEFAULT_KEYWORDS.map(|(k, v)| (String::from(k), String::from(v))),
            ),
            state: VMState::Halted,
            memory: Rc::clone(&memory),
            micro_mem: Rc::clone(&micro_mem),
            cpu: Cpu::new(Rc::clone(&memory), Rc::clone(&micro_mem)),
            initial_memory: None,
            microinstructions: Vec::new(),
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
    pub fn advance_microinstruction(&mut self) -> (usize, usize, MachineEvents) {
        match &self.state {
            VMState::Active => self.cpu.advance_microinstruction(),

            _ => Default::default(),
        }
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
    pub fn get_control_signals(&self) -> &ControlSignals {
        self.cpu.get_control_signals()
    }
    pub fn get_registors(&self) -> (u16, u16, &RegistorBank) {
        self.cpu.get_registors()
    }
}
