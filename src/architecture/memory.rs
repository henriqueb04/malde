use crate::architecture::{events::EventHandler, signals::ControlSignals};
use log::{info, warn};

pub type MemoryArray = [u16; MEMORY_SIZE];

pub const DATA_SEGMENT_START: usize = 1536;
pub const TEXT_SEGMENT_START: usize = 0;

pub const MEMORY_SIZE: usize = 1 << 12;
pub struct Memory {
    rd_clock_count: u8,
    wr_clock_count: u8,
    previous_mar: u16,
    previous_mbr: u16,
    memory: MemoryArray,
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            rd_clock_count: 0,
            wr_clock_count: 0,
            previous_mar: 0,
            previous_mbr: 0,
            memory: [0; MEMORY_SIZE],
        }
    }

    pub fn clear(&mut self) {
        self.memory.fill(0);
        self.rd_clock_count = 0;
        self.wr_clock_count = 0;
        self.previous_mar = 0;
    }

    pub fn load(&mut self, start: usize, data: &[u16]) {
        let end = usize::min(self.memory.len(), start + data.len());
        let len = end - start;
        self.memory[start..start + len].copy_from_slice(&data[..len]);
    }

    pub fn request_rd(&mut self, mar: &u16, mbr: &mut u16, events: &mut EventHandler) {
        self.rd_clock_count += 1;
        if self.rd_clock_count < 2 {
            if *mar < DATA_SEGMENT_START as u16 {
                events.instruction_read(*mar);
            }
            return;
        }
        if self.previous_mar != *mar {
            events.mar_conflict(self.previous_mar, *mar);
            info!("mar conflict")
        } else {
            let before = *mbr;
            *mbr = self.memory[*mar as usize];
            events.mbr_write(before, *mbr);
        }
    }

    fn request_wr(&mut self, mar: &u16, mbr: &mut u16, events: &mut EventHandler) {
        self.wr_clock_count += 1;
        if self.wr_clock_count < 2 {
            return;
        }
        if self.previous_mar != *mar {
            events.mar_conflict(self.previous_mar, *mar);
            info!("mar conflict")
        } else if self.previous_mbr != *mbr {
            events.mbr_conflict(self.previous_mbr, *mbr);
            info!("mbr conflict")
        } else {
            let before = self.memory[self.previous_mar as usize];
            self.memory[*mar as usize] = *mbr;
            events.memory_write(self.previous_mar, before, self.memory[*mar as usize]);
        }
    }

    pub fn clock(
        &mut self,
        signals: &ControlSignals,
        mar: &u16,
        mbr: &mut u16,
        events: &mut EventHandler,
    ) {
        if *mar >= MEMORY_SIZE as u16 {
            warn!("Endereço {} é maior que memória! Ignorando...", mar);
        }
        let rd = &signals.rd;
        let wr = &signals.wr;
        info!("{}, {}", rd, wr);
        if !rd && !wr {
            self.rd_clock_count = 0;
            self.wr_clock_count = 0;
            return;
        }
        if *wr {
            self.request_wr(mar, mbr, events);
            self.previous_mar = *mar;
            self.previous_mbr = *mbr;
        }
        if *rd {
            self.request_rd(mar, mbr, events);
            self.previous_mar = *mar;
        }
    }

    pub fn get_ref(&self) -> &MemoryArray {
        &self.memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn load_test() {
        let mut mem = Memory::new();
        mem.load(5, &[1, 2, 3, 4, 5]);
        assert_eq!(mem.memory[5], 1);
        assert_eq!(mem.memory[6], 2);
        assert_eq!(mem.memory[7], 3);
        assert_eq!(mem.memory[8], 4);
        assert_eq!(mem.memory[9], 5);
    }
}
