pub struct ALUSignals {
    pub z: bool,
    pub n: bool,
}

#[derive(Default)]
pub struct ControlSignals {
    pub amux: bool,
    pub cond: u8,
    pub alu: u8,
    pub sh: u8,
    pub mbr: bool,
    pub mar: bool,
    pub rd: bool,
    pub wr: bool,
    pub enc: bool,
    pub c: u8,
    pub b: u8,
    pub a: u8,
    pub addr: u8,
}
