use std::{
    collections::HashSet,
    fmt::Debug,
    mem::discriminant,
    sync::{Arc, Mutex},
};

use crossbeam_channel::{Receiver, Sender, unbounded};
use thiserror::Error;

use crate::{
    architecture::{
        Cpu,
        control::{MICROMEM_MAX_SIZE, MicroMem},
        datapath::DataRegisters,
        events::EventHandler,
        memory::{Memory, MemoryArray},
    },
    parsers::{
        asm::{ASMParser, ASMParserResult, ASMParsingError, Instruction, KeywordMap},
        mal::{MALParser, MALParsingError, Microinstruction},
        source_map::SourceMap,
    },
};

pub use crate::architecture::memory::{DATA_SEGMENT_START, TEXT_SEGMENT_START};
pub use crate::architecture::{datapath::Register, memory::MEMORY_SIZE};

pub struct VM {
    state: VMState,
    execution_type: Option<VMExecutionType>,
    initial_memory: Option<(Vec<u16>, Vec<u16>)>,
    memory: Arc<Mutex<Memory>>,
    micro_mem: Arc<Mutex<MicroMem>>,
    cpu: Cpu,
    instructions: Vec<Instruction>,
    microinstructions: Vec<Microinstruction>,
    events: EventHandler,
    pc: usize,
    prev_pc: usize,
    stdout: String,
    info_print: bool,
    on_print: Option<Box<dyn Fn(String) + Send>>,
    err_on_instruction_write: bool,
    // cur_instruction: usize,
}

impl VM {
    pub fn new() -> Self {
        let memory = Arc::new(Mutex::new(Memory::new()));
        let micro_mem = Arc::new(Mutex::new(MicroMem::new(Vec::new())));
        VM {
            memory: Arc::clone(&memory),
            micro_mem: Arc::clone(&micro_mem),
            cpu: Cpu::new(Arc::clone(&memory), Arc::clone(&micro_mem)),
            state: VMState::Uninitialized,
            execution_type: None,
            microinstructions: Vec::new(),
            instructions: Vec::new(),
            initial_memory: None,
            events: EventHandler::default(),
            pc: 0,
            prev_pc: 0,
            stdout: String::new(),
            info_print: true,
            on_print: None,
            err_on_instruction_write: false,
        }
    }

    pub fn assemble_mic(
        &mut self,
        source_map: &SourceMap,
    ) -> Result<Vec<Microinstruction>, VMError> {
        let parser = MALParser::new(source_map);
        let microinstructions = parser.parse()?;
        if microinstructions.len() > MICROMEM_MAX_SIZE {
            return Err(VMError::MicroMemOverflow(microinstructions.len()));
        }
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

    pub fn assemble_mac(
        &mut self,
        source_map: &SourceMap,
        keywords: KeywordMap,
    ) -> Result<Vec<Instruction>, VMError> {
        let parser = ASMParser::new(source_map, keywords, DATA_SEGMENT_START);
        let ASMParserResult {
            data_mem,
            ins_mem,
            instructions,
        } = parser.parse()?;
        if data_mem.len() > MEMORY_SIZE - DATA_SEGMENT_START {
            return Err(VMError::DataSegmentOverflow(data_mem.len()));
        }
        if ins_mem.len() > DATA_SEGMENT_START {
            return Err(VMError::InstructionSegmentOverflow(ins_mem.len()));
        }
        self.set_initial_memory(ins_mem, data_mem);
        self.instructions = instructions;
        self.reset();
        Ok(self.instructions.clone())
    }

    const MIN_INT: isize = i16::MIN as isize;
    const MAX_INT: isize = u16::MAX as isize;
    fn handle_input(&mut self, input_type: VMInputResponse) -> Result<(), VMInputError> {
        let res: Result<(), VMInputError> = if let VMState::Waiting(request_type) = &self.state {
            match (input_type, request_type) {
                (VMInputResponse::Int(n), VMInputRequestType::Int) => {
                    if (VM::MIN_INT..=VM::MAX_INT).contains(&n) {
                        self.cpu
                            .set_register(Register::Ac.index().unwrap(), n as u16);
                        Ok(())
                    } else {
                        Err(VMInputError::InvalidNumber(n, VM::MIN_INT, VM::MAX_INT))
                    }
                }
                (VMInputResponse::Char(c), VMInputRequestType::Char) => {
                    self.cpu
                        .set_register(Register::Ac.index().unwrap(), c as u8 as u16);
                    Ok(())
                }
                (VMInputResponse::String(s), VMInputRequestType::String) => {
                    let addr = self.registers().2[Register::A.index().unwrap()] as usize;
                    let max_size = self.registers().2[Register::B.index().unwrap()] as usize;
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
                    self.cpu.set_register(
                        Register::Ac.index().unwrap(),
                        usize::min(size + 1, max_size) as u16,
                    );
                    Ok(())
                }
                _ => Err(VMInputError::WrongType),
            }
        } else {
            Err(VMInputError::Unexpected)
        };
        if res.is_ok() {
            self.state = VMState::Active;
        }
        res
    }

    pub fn state(&self) -> &VMState {
        &self.state
    }

    pub fn set_err_on_instruction_write(&mut self, err_on_instruction_write: bool) {
        self.err_on_instruction_write = err_on_instruction_write;
    }
    pub fn set_info_print(&mut self, info_print: bool) {
        self.info_print = info_print;
    }
    pub fn set_on_print(&mut self, on_print: Box<dyn Fn(String) + Send>) {
        self.on_print = Some(on_print);
    }
    fn print_to_stdout(&mut self, s: &str) {
        self.stdout.push_str(s);
        if let Some(on_print) = &self.on_print {
            (on_print)(s.to_string());
        }
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
            memory.load(DATA_SEGMENT_START, initial_data);
        }
    }
    pub fn memory(&self) -> MemoryArray {
        let memory = self.memory.lock().unwrap();
        *memory.get_ref()
    }

    // Cpu
    pub fn execute(
        &mut self,
        execution_type: VMExecutionType,
        execution_info: VMExecutionInfo,
    ) -> Box<Result<VMResponse, (VMResponse, VMError)>> {
        self.events.clear();
        self.execution_type = Some(execution_type.clone());
        let (res, r) = match &self.state {
            VMState::Active => match execution_type {
                VMExecutionType::Run => self.run_all(&execution_info),
                VMExecutionType::Macroinstruction => self.advance_macroinstruction(&execution_info),
                VMExecutionType::Microinstruction => self.advance_microinstruction(&execution_info),
            },
            _ => ((0, 0), Ok(())),
        };
        if discriminant(&self.state) != discriminant(&VMState::Waiting(VMInputRequestType::Int)) {
            self.execution_type = None;
        }
        if r.is_err() {
            self.state = VMState::Halted;
        }
        let res = VMResponse {
            mpc: res.0,
            prev_mpc: res.1,
            pc: self.pc,
            prev_pc: self.prev_pc,
            events: self.events.clone(),
            state: self.state.clone(),
            registers: self.registers(),
        };
        Box::new(if let Err(err) = r {
            Err((res, err))
        } else {
            Ok(res)
        })
    }
    fn run_all(
        &mut self,
        execution_info: &VMExecutionInfo,
    ) -> ((usize, usize), Result<(), VMError>) {
        let mut res = (0, 0);
        let mut r;
        while self.state == VMState::Active {
            self.events.instruction_reads.clear();
            (res, r) = self.advance_microinstruction(execution_info);
            if let Err(err) = r {
                return (res, Err(err));
            }
            if let Some(pc) = self.events.instruction_reads.iter().next()
                && execution_info.breaks_mac.contains(&(*pc as usize))
            {
                break;
            }
            if execution_info.breaks_mic.contains(&res.0) {
                break;
            }
            if execution_info.r_pause.try_recv().is_ok() {
                break;
            }
        }
        (res, Ok(()))
    }
    fn advance_microinstruction(
        &mut self,
        execution_info: &VMExecutionInfo,
    ) -> ((usize, usize), Result<(), VMError>) {
        match &self.state {
            VMState::Active => {
                let (mpc, prev_mpc) = self.cpu.advance_microinstruction(&mut self.events);
                if self.events.mar_conflicting.is_some() {
                    return ((mpc, prev_mpc), Err(VMError::MarChanged));
                }
                if self.events.mbr_conflicting.is_some() {
                    return ((mpc, prev_mpc), Err(VMError::MbrChanged));
                }
                if self.err_on_instruction_write
                    && let Some(addr) = self.events.instruction_writes.iter().next()
                {
                    return ((mpc, prev_mpc), Err(VMError::InstructionWrite(*addr)));
                }
                if self.microinstructions[prev_mpc].mir.syscall
                    && let Some(input_request) = self.execute_syscall()
                {
                    let (input_s, input_r) = unbounded::<VMInputResponse>();
                    (execution_info.on_input_request)(VMInputRequest {
                        typ: input_request,
                        sender: input_s,
                    });
                    loop {
                        let Ok(input) = input_r.recv() else {
                            self.state = VMState::Halted;
                            return (Default::default(), Ok(()));
                        };
                        match self.handle_input(input).map_err(|err| err.to_string()) {
                            Ok(..) => {
                                (execution_info.on_validation)(Ok(()));
                                break;
                            }
                            Err(err) => (execution_info.on_validation)(Err(err.to_string())),
                        }
                    }
                };
                if !self.events.instruction_reads.is_empty() {
                    self.prev_pc = self.pc;
                    self.pc = self.cpu.get_registers().pc() as usize;
                }
                ((mpc, prev_mpc), Ok(()))
            }
            _ => (Default::default(), Ok(())),
        }
    }
    fn advance_macroinstruction(
        &mut self,
        execution_info: &VMExecutionInfo,
    ) -> ((usize, usize), Result<(), VMError>) {
        let mut res = (0, 0);
        let mut r;
        while self.events.instruction_reads.is_empty() && self.state == VMState::Active {
            (res, r) = self.advance_microinstruction(execution_info);
            if let Err(err) = r {
                return (res, Err(err));
            }
            if execution_info.breaks_mic.contains(&res.0) {
                break;
            }
            if execution_info.r_pause.try_recv().is_ok() {
                break;
            }
        }
        (res, Ok(()))
    }

    pub fn reset(&mut self) {
        self.pc = 0;
        self.prev_pc = 0;
        self.events.clear();
        self.reset_memory();
        self.cpu.reset();
        if self.state != VMState::Uninitialized {
            if self.state == VMState::Halted {
                self.state = VMState::Active;
            }
            if self.info_print {
                self.print_to_stdout("\n\n----- programa reiniciado -----\n\n");
            }
        }
    }
    pub fn registers(&self) -> DataRegisters {
        self.cpu.get_registers()
    }

    fn execute_syscall(&mut self) -> Option<VMInputRequestType> {
        let registers = self.cpu.get_registers();
        match registers.ac() {
            Syscalls::PRINT_INT => {
                let s = format!("{}", registers.a() as i16);
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_CHAR => {
                let s = format!("{}", registers.a() as u8 as char);
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_INT_HEX => {
                let s = format!("{:04X}", registers.a());
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_STRING => {
                let start = registers.a();
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
                let s = format!("{:016b}", registers.a());
                self.print_to_stdout(&s);
                None
            }
            Syscalls::PRINT_INT_UNSIGNED => {
                let s = format!("{}", registers.a());
                self.print_to_stdout(&s);
                None
            }
            Syscalls::READ_INT => {
                self.state = VMState::Waiting(VMInputRequestType::Int);
                Some(VMInputRequestType::Int)
            }
            Syscalls::READ_CHAR => {
                self.state = VMState::Waiting(VMInputRequestType::Char);
                Some(VMInputRequestType::Char)
            }
            Syscalls::READ_STRING => {
                self.state = VMState::Waiting(VMInputRequestType::String);
                Some(VMInputRequestType::String)
            }
            Syscalls::HALT => {
                if self.info_print {
                    self.print_to_stdout("\n\n----- programa encerrado (0) -----\n\n");
                }
                self.state = VMState::Halted;
                None
            }
            _ => None,
        }
    }
}

impl Debug for VM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VM")
            .field("state", &self.state)
            .field("execution_type", &self.execution_type)
            .field("cpu", &self.cpu)
            .field("events", &self.events)
            .field("stdout", &self.stdout)
            .finish()
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
pub enum VMInputRequestType {
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
    pub pc: usize,
    pub prev_pc: usize,
    pub events: EventHandler,
    pub state: VMState,
    pub registers: DataRegisters,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum VMState {
    #[default]
    Uninitialized,
    Active,
    Waiting(VMInputRequestType),
    Halted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMExecutionType {
    Run,
    Macroinstruction,
    Microinstruction,
}

#[derive(Debug)]
pub struct VMInputRequest {
    pub typ: VMInputRequestType,
    pub sender: Sender<VMInputResponse>,
}

pub struct VMExecutionInfo {
    pub r_pause: Receiver<()>,
    pub on_input_request: Box<dyn Fn(VMInputRequest) + Send>,
    pub on_validation: Box<dyn Fn(Result<(), String>) + Send>,
    pub breaks_mic: HashSet<usize>,
    pub breaks_mac: HashSet<usize>,
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

#[derive(Debug, Error)]
#[error(transparent)]
pub enum VMError {
    #[error(transparent)]
    ASMParsingError(#[from] Box<ASMParsingError>),
    #[error(transparent)]
    MALParsingError(#[from] Box<MALParsingError>),
    #[error("MAR mudou durante operação da memória. Risco de corrupção de memória.")]
    MarChanged,
    #[error("MBR mudou durante operação de escrita. Risco de corrupção de memória.")]
    MbrChanged,
    #[error(
        "Tentativa de escrita no segmento de instrução, que é somente leitura durante execução."
    )]
    InstructionWrite(u16),
    #[error("Quantidade de instruções excede a capacidade do segmento de instruções.")]
    InstructionSegmentOverflow(usize),
    #[error("Quantidade de instruções excede a capacidade do segmento de dados.")]
    DataSegmentOverflow(usize),
    #[error("Quantidade de microinstruções excede a capacidade da memória de microinstrução.")]
    MicroMemOverflow(usize),
}
