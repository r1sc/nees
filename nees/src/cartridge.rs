pub trait Cartridge {
    fn ppu_read(&mut self, address: u16, ciram: &[u8]) -> u8;
    fn ppu_write(&mut self, address: u16, value: u8, ciram: &mut [u8]);
    fn cpu_read(&self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, value: u8);
    fn scanline(&mut self) -> bool;

    fn save(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()>;
    fn load(&mut self, reader: &mut dyn std::io::Read) -> std::io::Result<()>;
}
