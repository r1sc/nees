
pub const BIT_13: u16 = 1 << 13;

pub const MASK_32K: u16 = 0x7FFF;
pub const MASK_16K: u16 = 0x3FFF;

pub trait SubType<T> {
    fn lower_4k(&self) -> T;
    fn lower_8k(&self) -> T;
    fn lower_16k(&self) -> T;
    fn lower_32k(&self) -> T;
}

impl SubType<u16> for u16 {
    fn lower_4k(&self) -> u16 {
        self & 0x0FFF
    }
    
    fn lower_8k(&self) -> u16 {
        self & 0x1FFF
    }

    fn lower_16k(&self) -> u16 {
        self & 0x3FFF
    }

    fn lower_32k(&self) -> u16 {
        self & 0x7FFF
    }
}