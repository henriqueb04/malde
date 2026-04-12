use crate::architecture::signals::ControlSignals;

const MEMORY_SIZE: u16 = (1 << 12) - 1;
pub struct Memory {
    rd_clock_count: u8,
    wr_clock_count: u8,
    previous_mar: u16,
    memory: [u16; MEMORY_SIZE as usize],
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            rd_clock_count: 0,
            wr_clock_count: 0,
            previous_mar: 0,
            memory: [0; MEMORY_SIZE as usize],
        }
    }

    pub fn request_rd(&mut self, mar: &u16, mbr: &mut u16) {
        self.rd_clock_count += 1;
        if self.rd_clock_count < 2 {
            return;
        }
        if self.previous_mar != *mar {
            println!(
                "MAR address changed from {} to {} before response from memory! Ignoring...",
                self.previous_mar, mar
            );
            *mbr = self.memory[self.previous_mar as usize];
        } else {
            *mbr = self.memory[*mar as usize];
        }
    }

    fn request_wr(&mut self, mar: &u16, mbr: &mut u16) {
        self.wr_clock_count += 1;
        if self.wr_clock_count < 2 {
            return;
        }
        if self.previous_mar != *mar {
            println!(
                "MAR address changed from {} to {} before response from memory! Ignoring...",
                self.previous_mar, mar
            );
            self.memory[self.previous_mar as usize] = *mbr;
        } else {
            self.memory[*mar as usize] = *mbr;
        }
    }

    pub fn clock(&mut self, signals: &ControlSignals, mar: &u16, mbr: &mut u16) {
        if *mar >= MEMORY_SIZE {
            println!("Address {} is out of bounds! Ignoring...", mar);
        }
        let rd = &signals.rd;
        let wr = &signals.wr;
        if !rd && !wr {
            self.rd_clock_count = 0;
            self.wr_clock_count = 0;
            return;
        } else if *rd && *wr {
            println!("Both RW and WR are on at the same time!");
        }
        if *wr {
            self.request_wr(mar, mbr);
            self.previous_mar = *mar;
        }
        if *rd {
            self.request_rd(mar, mbr);
            self.previous_mar = *mar;
        }
    }
}
