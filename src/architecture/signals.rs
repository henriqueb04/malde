#[inline]
fn slice_bits(bits: &u32, start: usize, end: usize) -> u8 {
    let len = end - start;
    let mask: u32 = (1 << len) - 1;
    ((bits >> (32 - end)) & mask) as u8
}

#[inline]
fn get_bit(bits: &u32, i: u8) -> bool {
    ((bits >> (31 - i)) & 1) == 1
}

#[inline]
fn position_bits(n: &u8, start: usize, end: usize) -> u32 {
    let len = end - start;
    let mask: u32 = (1 << len) - 1;
    ((*n as u32) & mask).overflowing_shl(32 - end as u32).0
}

#[inline]
fn position_bit(b: &bool, i: u8) -> u32 {
    (*b as u32) << 31 - i
}

pub struct ALUSignals {
    pub z: bool,
    pub n: bool,
}

#[derive(Default, Clone, PartialEq, Debug)]
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

pub const CONTROL_SIGNAL_NAMES_B: [&'static str; 6] = [
    "amux", "mbr", "mar", "rd", "wr", "enc",
];
pub const CONTROL_SIGNAL_NAMES_U: [&'static str; 7] = [
    "cond", "alu", "sh", "c", "b", "a", "addr",
];

impl From<&u32> for ControlSignals {
    #[rustfmt::skip]
    fn from(n: &u32) -> Self {
        ControlSignals {
            amux: get_bit(n, 0),
            cond: slice_bits(n, 1, 3),
            alu : slice_bits(n, 3, 5),
            sh  : slice_bits(n, 5, 7),
            mbr : get_bit(n, 7),
            mar : get_bit(n, 8),
            rd  : get_bit(n, 9),
            wr  : get_bit(n, 10),
            enc : get_bit(n, 11),
            c   : slice_bits(n, 12, 16),
            b   : slice_bits(n, 16, 20),
            a   : slice_bits(n, 20, 24),
            addr: slice_bits(n, 24, 32),
        }
    }
}

impl From<ControlSignals> for u32 {
    fn from(item: ControlSignals) -> u32 {
        position_bit(&item.amux, 0) |
        position_bits(&item.cond, 1, 3) |
        position_bits(&item.alu, 3, 5) |
        position_bits(&item.sh, 5, 7) |
        position_bit(&item.mbr, 7) |
        position_bit(&item.mar, 8) |
        position_bit(&item.rd, 9) |
        position_bit(&item.wr, 10) |
        position_bit(&item.enc, 11) |
        position_bits(&item.c, 12, 16) |
        position_bits(&item.b, 16, 20) |
        position_bits(&item.a, 20, 24) |
        position_bits(&item.addr, 24, 32)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_converting() {
        let sigs = ControlSignals {
            amux: true,
            cond: 0b01,
            alu: 0b10,
            sh: 0b00,
            mbr: true,
            mar: false,
            rd: true,
            wr: false,
            enc: false,
            c: 0b1001,
            b: 0b0110,
            a: 0b1111,
            addr: 0b10101001,
        };
        let n: u32 = sigs.clone().into();
        let expected = 0b1_01_10_00_10100_1001_0110_1111_10101001;
        println!("expected: {:b}", expected);
        println!("result  : {:b}", n);
        assert_eq!(n, expected);
        assert_eq!(ControlSignals::from(&n), sigs);

        let sigs = ControlSignals {
            amux: false,
            cond: 0b11,
            alu: 0b01,
            sh: 0b11,
            mbr: false,
            mar: true,
            rd: true,
            wr: false,
            enc: true,
            c: 0b0101,
            b: 0b0000,
            a: 0b0001,
            addr: 0b01111110,
        };
        let n: u32 = sigs.clone().into();
        let expected = 0b0_11_01_11_01101_0101_0000_0001_01111110;
        println!("expected: {:b}", expected);
        println!("result  : {:b}", n);
        assert_eq!(n, expected);
        assert_eq!(ControlSignals::from(&n), sigs);
    }
}
