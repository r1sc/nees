pub trait Cartridge {
    fn ppu_read(&self, address: u16, ciram: &[u8]) -> u8;
    fn ppu_write(&mut self, address: u16, value: u8, ciram: &mut [u8]);
    fn cpu_read(&self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, value: u8);
}
