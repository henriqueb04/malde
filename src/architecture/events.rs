#[derive(Default, Debug, PartialEq, Eq)]
pub struct MachineEvents {
    pub memory_read_start: Option<SlotReadEvent>,
    pub memory_changed: Option<SlotChangeEvent>,
    pub register_changed: Option<SlotChangeEvent>,
    pub mar_changed: Option<NamedChangeEvent>,
    pub mbr_changed: Option<NamedChangeEvent>,
    pub syscall: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotChangeEvent {
    pub slot: usize,
    pub before: u16,
    pub after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedChangeEvent {
    pub before: u16,
    pub after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotReadEvent {
    pub slot: usize,
}

impl MachineEvents {
    pub fn new() -> Self {
        Self::default()
    }
}
