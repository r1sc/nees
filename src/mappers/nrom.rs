use crate::{cartridge::Cartridge, ines::INES};

pub struct NROM {
    ines: INES,
}

impl NROM {    
    pub fn new(ines: INES) -> Self {
        Self { ines }
    }

    fn nrom_ppu_addr_to_ciram_addr(&self, ppuaddr: u16) -> u16 {
        (ppuaddr & 0x3ff) | (((ppuaddr >> self.ines.ppu_address_ciram_a10_shift_count) & 1) << 10)
    }
}

const BIT_13: u16 = 1 << 13;

impl Cartridge for NROM {
    fn ppu_read(&self, address: u16, ciram: &[u8]) -> u8 {
        if (address & BIT_13) == BIT_13 {
            ciram[self.nrom_ppu_addr_to_ciram_addr(address) as usize]
        } else {
            self.ines.chr_rom[(address & 0x1fff) as usize]
        }
    }

    fn ppu_write(&mut self, address: u16, value: u8, ciram: &mut [u8]) {
        if (address & BIT_13) == BIT_13 {
            ciram[self.nrom_ppu_addr_to_ciram_addr(address) as usize] = value;
        }
    }

    fn cpu_read(&self, address: u16) -> u8 {
        self.ines.prg_rom[(address & (if self.ines.prg_rom_size_16k_chunks == 1 { 0x3FFF } else { 0x7FFF })) as usize]
    }

    fn cpu_write(&mut self, _address: u16, _value: u8) {
        // Do nothing
    }

}
