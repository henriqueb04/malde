#[derive(Debug, PartialEq, Eq)]
pub struct MachineEvents {
    pub memory_changed: Option<SlotChangeEvent>,
    pub registor_changed: Option<SlotChangeEvent>,
    pub mar_changed: Option<NamedChangeEvent>,
    pub mbr_changed: Option<NamedChangeEvent>,
    pub syscall: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SlotChangeEvent {
    pub slot: usize,
    pub before: u16,
    pub after: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NamedChangeEvent {
    pub before: u16,
    pub after: u16,
}

impl MachineEvents {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for MachineEvents {
    fn default() -> Self {
        MachineEvents {
            memory_changed: None,
            registor_changed: None,
            mar_changed: None,
            mbr_changed: None,
            syscall: false,
        }
    }
}
