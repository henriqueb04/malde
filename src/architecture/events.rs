use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteEvent {
    pub before: u16,
    pub after: u16,
}

#[derive(Debug, Default)]
pub struct EventHandler {
    pub register_writes: HashMap<u8, WriteEvent>,
    pub memory_writes: HashMap<u16, WriteEvent>,
    pub instruction_reads: HashSet<u16>,
    pub mar_conflicting: Option<WriteEvent>,
    pub mbr_conflicting: Option<WriteEvent>,
    pub mar_written: Option<WriteEvent>,
    pub mbr_written: Option<WriteEvent>,
}

impl EventHandler {
    pub fn memory_write(&mut self, addr: u16, before: u16, after: u16) {
        if let Some(ev) = self.memory_writes.get_mut(&addr) {
            ev.after = after;
        } else {
            self.memory_writes
                .insert(addr, WriteEvent { before, after });
        }
    }

    pub fn register_write(&mut self, i: u8, before: u16, after: u16) {
        if let Some(ev) = self.register_writes.get_mut(&i) {
            ev.after = after;
        } else {
            self.register_writes.insert(i, WriteEvent { before, after });
        }
    }

    pub fn instruction_read(&mut self, addr: u16) {
        self.instruction_reads.insert(addr);
    }

    pub fn mar_conflict(&mut self, before: u16, after: u16) {
        self.mar_conflicting = Some(WriteEvent { before, after })
    }
    pub fn mbr_conflict(&mut self, before: u16, after: u16) {
        self.mbr_conflicting = Some(WriteEvent { before, after })
    }

    pub fn mar_write(&mut self, before: u16, after: u16) {
        self.mar_written = Some(WriteEvent { before, after })
    }
    pub fn mbr_write(&mut self, before: u16, after: u16) {
        self.mbr_written = Some(WriteEvent { before, after })
    }

    pub fn clear(&mut self) {
        self.register_writes.clear();
        self.memory_writes.clear();
        self.instruction_reads.clear();
        self.mar_conflicting = None;
        self.mbr_conflicting = None;
        self.mar_written = None;
        self.mbr_written = None;
    }
}
