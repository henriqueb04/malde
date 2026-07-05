use thiserror::Error;
use log::info;

use crate::architecture::datapath::RegisterBank;
use crate::architecture::events::EventHandler;
use crate::architecture::memory::{Memory, MemoryArray};
use std::cell::Ref;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    architecture::{Cpu, control::MicroMem},
    parsers::{
        mac::{ASMParser, DEFAULT_KEYWORDS, ParsingError as ASMParsingError},
        mal::{MALParser, Microinstruction, ParsingError as MALParsingError},
    },
};

pub use crate::architecture::memory::{DATA_SEGMENT_START, TEXT_SEGMENT_START};
pub use crate::architecture::{datapath::Registers, memory::MEMORY_SIZE};

pub struct VM {
    keywords: HashMap<String, String>,
    state: VMState,
    initial_memory: Option<(Vec<u16>, Vec<u16>)>,
    memory: Rc<RefCell<Memory>>,
    micro_mem: Rc<RefCell<MicroMem>>,
    cpu: Cpu,
    microinstructions: Vec<Microinstruction>,
    events: EventHandler,
    stdout: String,
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
            state: VMState::Uninitialized,
            microinstructions: Vec::new(),
            initial_memory: None,
            events: EventHandler::default(),
            stdout: String::new(),
        }
    }

    pub fn print_to_stdout(&mut self, s: &str) {
        self.stdout.push_str(s);
        print!("{}", s);
    }
    pub fn get_stdout(&self) -> &String {
        &self.stdout
    }
    pub fn get_events(&self) -> &EventHandler {
        &self.events
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

    const MIN_INT: isize = i16::MIN as isize;
    const MAX_INT: isize = u16::MAX as isize;
    pub fn handle_input(&mut self, input_type: VMInputResponse) -> Result<(), VMInputError> {
        let res = if let VMState::Waiting(request_type) = &self.state {
            match (input_type, request_type) {
                (VMInputResponse::Int(n), VMInputRequest::Int) => {
                    if (VM::MIN_INT..=VM::MAX_INT).contains(&n) {
                        self.cpu.set_register(Registers::AC, n as u16);
                        Ok(())
                    } else {
                        Err(VMInputError::InvalidNumber(n, VM::MIN_INT, VM::MAX_INT))
                    }
                }
                (VMInputResponse::Char(c), VMInputRequest::Char) => {
                    self.cpu.set_register(Registers::AC, c as u8 as u16);
                    Ok(())
                }
                (VMInputResponse::String(s), VMInputRequest::String) => {
                    let ac = self.registers().2[Registers::AC] as usize;
                    let mut memory = self.memory.borrow_mut();
                    for (i, c) in s.chars().enumerate() {
                        memory.set_addr(ac + i, c as u8 as u16);
                    }
                    Ok(())
                }
                _ => Err(VMInputError::WrongType)
            }
        } else {
            Err(VMInputError::Unexpected)
        };
        if res.is_ok() {
            self.state = VMState::Active;
        }
        res
    }

    pub fn microinstructions(&self) -> &Vec<Microinstruction> {
        &self.microinstructions
    }

    pub fn is_ready(&self) -> bool {
        self.state != VMState::Uninitialized
    }
    pub fn is_waiting(&self) -> bool {
        if let VMState::Waiting(..) = self.state {
            return true;
        };
        false
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
    fn advance_microinstruction_no_clear_events(&mut self) -> VMResponse {
        match &self.state {
            VMState::Active => {
                let (mpc, prev_mpc) = self.cpu.advance_microinstruction(&mut self.events);
                let request = if self.microinstructions()[prev_mpc].mir.syscall {
                    self.execute_syscall()
                } else {
                    None
                };
                VMResponse {
                    mpc,
                    prev_mpc,
                    request,
                }
            }
            _ => Default::default(),
        }
    }
    pub fn advance_microinstruction(&mut self) -> VMResponse {
        self.events.clear();
        self.advance_microinstruction_no_clear_events()
    }
    fn advance_macroinstruction_no_clear_events(&mut self) -> VMResponse {
        let mut res = VMResponse::default();
        while self.events.instruction_reads.is_empty() && self.state == VMState::Active {
            res = self.advance_microinstruction_no_clear_events();
        }
        res
    }
    pub fn advance_macroinstruction(&mut self) -> VMResponse {
        self.events.clear();
        self.advance_macroinstruction_no_clear_events()
    }

    pub fn reset(&mut self) {
        self.events.clear();
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
    pub fn registers(&self) -> (u16, u16, &RegisterBank) {
        self.cpu.get_registers()
    }

    fn execute_syscall(&mut self) -> Option<VMInputRequest> {
        let (_, _, registers) = self.cpu.get_registers();
        match registers[Registers::E] {
            Syscalls::PRINT_INT => {
                let s = format!("{}", registers[Registers::AC] as i16);
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_CHAR => {
                let s = format!("{}", registers[Registers::AC] as u8 as char);
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_INT_HEX => {
                let s = format!("0x{:04X}", registers[Registers::AC]);
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_INT_BINARY => {
                let s = format!("0x{:04b}", registers[Registers::AC]);
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_INT_UNSIGNED => {
                let s = format!("{}", registers[Registers::AC]);
                self.print_to_stdout(&s);
                None
            }
            Syscalls::READ_INT => {
                self.state = VMState::Waiting(VMInputRequest::Int);
                Some(VMInputRequest::Int)
            }
            Syscalls::READ_CHAR => {
                self.state = VMState::Waiting(VMInputRequest::Char);
                Some(VMInputRequest::Char)
            }
            Syscalls::READ_STRING => {
                self.state = VMState::Waiting(VMInputRequest::String);
                Some(VMInputRequest::String)
            }
            Syscalls::HALT => {
                self.state = VMState::Halted;
                None
            }
            _ => None,
        }
    }
}

pub struct Syscalls;
impl Syscalls {
    pub const PRINT_INT: u16 = 1;
    pub const PRINT_CHAR: u16 = 2;
    pub const PRINT_STRING: u16 = 3;
    pub const PRINT_INT_HEX: u16 = 4;
    pub const PRINT_INT_BINARY: u16 = 5;
    pub const PRINT_INT_UNSIGNED: u16 = 6;
    pub const READ_INT: u16 = 7;
    pub const READ_CHAR: u16 = 8;
    pub const READ_STRING: u16 = 9;
    pub const HALT: u16 = 10;
}

#[derive(Debug, PartialEq, Eq)]
pub enum VMInputRequest {
    Int,
    Char,
    String,
}

pub enum VMInputResponse {
    Int(isize),
    Char(char),
    String(String),
}

#[derive(Debug, Default)]
pub struct VMResponse {
    pub mpc: usize,
    pub prev_mpc: usize,
    pub request: Option<VMInputRequest>,
}

#[derive(Default, PartialEq, Eq)]
pub enum VMState {
    #[default]
    Uninitialized,
    Active,
    Waiting(VMInputRequest),
    Halted,
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum VMInputError {
    #[error("Número {0} fora dos limites ({1} a {2})")]
    InvalidNumber(isize, isize, isize),
    #[error("Tipo de input errado")]
    WrongType,
    #[error("Input inesperado")]
    Unexpected,
}
