
pub trait Bus {
    fn cpu_read(&mut self, address: u16) -> u8;
    fn cpu_write(&mut self, address: u16, value: u8);
}