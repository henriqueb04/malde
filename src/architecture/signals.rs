#[inline]
fn slice_bits(bits: &u64, start: usize, end: usize) -> u8 {
    let len = end - start;
    let mask: u64 = (1 << len) - 1;
    ((bits >> (64 - end)) & mask) as u8
}

#[inline]
fn slice_bits_u16(bits: &u64, start: usize, end: usize) -> u16 {
    let len = end - start;
    let mask: u64 = (1 << len) - 1;
    ((bits >> (64 - end)) & mask) as u16
}

#[inline]
fn get_bit(bits: &u64, i: u8) -> bool {
    ((bits >> (63 - i)) & 1) == 1
}

#[inline]
fn position_bits(n: &u8, start: usize, end: usize) -> u64 {
    let len = end - start;
    let mask: u64 = (1 << len) - 1;
    ((*n as u64) & mask).wrapping_shl(64 - end as u32)
}

#[inline]
fn position_bits_u16(n: &u16, start: usize, end: usize) -> u64 {
    let len = end - start;
    let mask: u64 = (1 << len) - 1;
    ((*n as u64) & mask).wrapping_shl(64 - end as u32)
}

#[inline]
fn position_bit(b: &bool, i: u8) -> u64 {
    (*b as u64) << (63 - i)
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
    pub addr: u16,
}

pub const CONTROL_SIGNAL_NAMES_B: [&str; 6] = ["amux", "mbr", "mar", "rd", "wr", "enc"];
pub const CONTROL_SIGNAL_NAMES_U: [&str; 6] = ["cond", "alu", "sh", "c", "b", "a"];

impl From<&u64> for ControlSignals {
    #[rustfmt::skip]
    fn from(n: &u64) -> Self {
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
            addr: slice_bits_u16(n, 24, 34),
        }
    }
}

impl From<ControlSignals> for u64 {
    fn from(item: ControlSignals) -> u64 {
        position_bit(&item.amux, 0)
            | position_bits(&item.cond, 1, 3)
            | position_bits(&item.alu, 3, 5)
            | position_bits(&item.sh, 5, 7)
            | position_bit(&item.mbr, 7)
            | position_bit(&item.mar, 8)
            | position_bit(&item.rd, 9)
            | position_bit(&item.wr, 10)
            | position_bit(&item.enc, 11)
            | position_bits(&item.c, 12, 16)
            | position_bits(&item.b, 16, 20)
            | position_bits(&item.a, 20, 24)
            | position_bits_u16(&item.addr, 24, 34)
    }
}

#[cfg(test)]
#[allow(clippy::unusual_byte_groupings)]
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
        let n: u64 = sigs.clone().into();
        let expected: u64 = 0b1_01_10_00_10100_1001_0110_1111_0010101001_000000000000000000000000000000;
        println!("expected: {:b}", expected);
        println!("result  : {:b}", n);
        assert_eq!(n, expected);
        assert_eq!(ControlSignals::from(&n), sigs);
        let back = ControlSignals::from(&expected);
        assert_eq!(back, sigs);

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
        let n: u64 = sigs.clone().into();
        let expected = 0b0_11_01_11_01101_0101_0000_0001_0001111110_000000000000000000000000000000;
        println!("expected: {:b}", expected);
        println!("result  : {:b}", n);
        assert_eq!(n, expected);
        assert_eq!(ControlSignals::from(&n), sigs);
        let back = ControlSignals::from(&expected);
        assert_eq!(back, sigs);
    }
}
