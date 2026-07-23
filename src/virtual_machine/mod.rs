use thiserror::Error;

use std::{
    mem::discriminant,
    sync::{Arc, Mutex},
};

use crate::{
    architecture::{
        Cpu,
        control::MicroMem,
        datapath::RegisterBank,
        events::EventHandler,
        memory::{Memory, MemoryArray},
    },
    parsers::{
        asm::{
            ASMParser, ASMParsingError, Instruction, KeywordMap, ParserResult as ASMParsingResult,
        },
        mal::{MALParser, Microinstruction, ParsingError as MALParsingError},
        source_map::SourceMap,
    },
};

pub use crate::architecture::memory::{DATA_SEGMENT_START, TEXT_SEGMENT_START};
pub use crate::architecture::{datapath::Registers, memory::MEMORY_SIZE};

pub struct VM {
    keywords: KeywordMap,
    state: VMState,
    execution_type: Option<VMExecutionType>,
    initial_memory: Option<(Vec<u16>, Vec<u16>)>,
    memory: Arc<Mutex<Memory>>,
    micro_mem: Arc<Mutex<MicroMem>>,
    cpu: Cpu,
    instructions: Vec<Instruction>,
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
        let memory = Arc::new(Mutex::new(Memory::new()));
        let micro_mem = Arc::new(Mutex::new(MicroMem::new(Vec::new())));
        VM {
            keywords: KeywordMap::default(),
            memory: Arc::clone(&memory),
            micro_mem: Arc::clone(&micro_mem),
            cpu: Cpu::new(Arc::clone(&memory), Arc::clone(&micro_mem)),
            state: VMState::Uninitialized,
            execution_type: None,
            microinstructions: Vec::new(),
            instructions: Vec::new(),
            initial_memory: None,
            events: EventHandler::default(),
            stdout: String::new(),
        }
    }

    pub fn stdout(&self) -> &String {
        &self.stdout
    }
    pub fn events(&self) -> &EventHandler {
        &self.events
    }

    pub fn assemble_mic<'a>(
        &mut self,
        source: &'a str,
    ) -> Result<Vec<Microinstruction>, MALParsingError<'a>> {
        let parser = MALParser::new();
        let microinstructions = parser.parse_instructions(source)?;
        {
            let mut micro_mem = self.micro_mem.lock().unwrap();
            *micro_mem = MicroMem::new(
                microinstructions
                    .iter()
                    .map(|m| m.mir.clone().into())
                    .collect(),
            );
        }
        self.reset();
        self.state = VMState::Active;
        self.microinstructions = microinstructions;
        Ok(self.microinstructions.clone())
    }

    pub fn assemble_mac<'a>(
        &mut self,
        source_map: &'a SourceMap,
    ) -> Result<Vec<Instruction>, ASMParsingError<'a>> {
        let parser = ASMParser::new(source_map, self.keywords.clone(), DATA_SEGMENT_START);
        let ASMParsingResult {
            data_mem,
            ins_mem,
            instructions,
        } = parser.parse()?;
        self.set_initial_memory(ins_mem, data_mem);
        self.instructions = instructions;
        self.reset();
        Ok(self.instructions.clone())
    }

    const MIN_INT: isize = i16::MIN as isize;
    const MAX_INT: isize = u16::MAX as isize;
    pub fn handle_input(
        &mut self,
        input_type: VMInputResponse,
    ) -> Result<Option<VMResponse>, VMInputError> {
        let res: Result<(), VMInputError> = if let VMState::Waiting(request_type) = &self.state {
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
                    let addr = self.registers().2[Registers::AC] as usize;
                    let max_size = self.registers().2[Registers::A] as usize;
                    let mut memory = self.memory.lock().unwrap();
                    let mut size = 0;
                    for (i, c) in s.as_bytes().iter().enumerate() {
                        size += 1;
                        if size >= max_size {
                            break;
                        }
                        memory.set_addr(addr + i, *c as u16, &mut self.events);
                    }
                    memory.set_addr(addr + usize::min(size, max_size - 1), 0, &mut self.events);
                    Ok(())
                }
                _ => Err(VMInputError::WrongType),
            }
        } else {
            Err(VMInputError::Unexpected)
        };
        res.map(|_| {
            self.state = VMState::Active;
            self.resume()
        })
    }

    pub fn microinstructions(&self) -> &Vec<Microinstruction> {
        &self.microinstructions
    }
    pub fn instructions(&self) -> &Vec<Instruction> {
        &self.instructions
    }

    pub fn is_ready(&self) -> bool {
        self.state != VMState::Uninitialized
    }
    pub fn is_active(&self) -> bool {
        self.state == VMState::Active
    }

    fn print_to_stdout(&mut self, s: &str) {
        self.stdout.push_str(s);
        print!("{}", s);
    }

    // Memory
    pub fn set_initial_memory(&mut self, initial_instructions: Vec<u16>, initial_data: Vec<u16>) {
        self.initial_memory = Some((initial_instructions, initial_data));
    }
    pub fn reset_memory(&mut self) {
        if let Some((initial_instructions, initial_data)) = self.initial_memory.as_ref() {
            let mut memory = self.memory.lock().unwrap();
            memory.clear();
            memory.load(TEXT_SEGMENT_START, initial_instructions);
            memory.load(DATA_SEGMENT_START - 1, &[0]); // HALT de segurança
            memory.load(DATA_SEGMENT_START, initial_data);
        }
    }
    pub fn memory(&self) -> MemoryArray {
        let memory = self.memory.lock().unwrap();
        *memory.get_ref()
    }

    // Cpu
    pub fn execute(&mut self, execution_type: VMExecutionType) -> VMResponse {
        self.events.clear();
        self.execution_type = Some(execution_type.clone());
        let r = match &self.state {
            VMState::Active => match execution_type {
                VMExecutionType::Macroinstruction => self.advance_macroinstruction(),
                VMExecutionType::Microinstruction => self.advance_microinstruction(),
            },
            _ => VMResponse::default(),
        };
        if discriminant(&self.state) != discriminant(&VMState::Waiting(VMInputRequest::Int)) {
            self.execution_type = None;
        }
        r
    }
    pub fn resume(&mut self) -> Option<VMResponse> {
        if let Some(execution_type) = &self.execution_type
            && *execution_type == VMExecutionType::Macroinstruction
        {
            Some(self.advance_macroinstruction())
        } else {
            None
        }
    }
    fn advance_microinstruction(&mut self) -> VMResponse {
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
    fn advance_macroinstruction(&mut self) -> VMResponse {
        let mut res = VMResponse::default();
        while self.events.instruction_reads.is_empty() && self.state == VMState::Active {
            res = self.advance_microinstruction();
        }
        res
    }

    pub fn reset(&mut self) {
        self.events.clear();
        self.reset_memory();
        self.cpu.reset();
        if self.state != VMState::Uninitialized {
            if self.state == VMState::Halted {
                self.state = VMState::Active;
            }
            self.print_to_stdout("\n\n----- programa reiniciado -----\n\n");
        }
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
                let s = format!("{:04X}", registers[Registers::AC]);
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_STRING => {
                let start = registers[Registers::AC];
                let s = {
                    let memory = self.memory.lock().unwrap();
                    let m = memory.get_ref();
                    let mut i = start as usize;
                    let mut s = String::new();
                    while m[i] != 0 {
                        s.push_str(&(format!("{}", m[i] as u8 as char)));
                        i += 1;
                    }
                    s
                };
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_INT_BINARY => {
                let s = format!("{:016b}", registers[Registers::AC]);
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
                self.print_to_stdout("\n\n----- programa encerrado (0) -----\n\n");
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMExecutionType {
    Macroinstruction,
    Microinstruction,
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
