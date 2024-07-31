use crate::{cartridge::Cartridge, ines::INES};

pub struct UNROM {
    ines: INES,
    selected_bank: u8,
}

impl UNROM {
    pub fn new(ines: INES) -> Self {
        Self {
            ines,
            selected_bank: 0,
        }
    }

    fn ppu_addr_to_ciram_addr(&self, ppuaddr: u16) -> u16 {
        (ppuaddr & 0x3ff) | (((ppuaddr >> self.ines.ppu_address_ciram_a10_shift_count) & 1) << 10)
    }
}

const BIT_13: u16 = 1 << 13;

impl Cartridge for UNROM {
    fn ppu_read(&self, address: u16, ciram: &[u8]) -> u8 {
        if (address & BIT_13) == BIT_13 {
            ciram[self.ppu_addr_to_ciram_addr(address) as usize]
        } else {
            self.ines.chr_rom[(address & 0x1fff) as usize]
        }
    }

    fn ppu_write(&mut self, address: u16, value: u8, ciram: &mut [u8]) {
        if (address & BIT_13) == BIT_13 {
            ciram[self.ppu_addr_to_ciram_addr(address) as usize] = value;
        }
        else if self.ines.is_chr_ram {
            self.ines.chr_rom[(address & 0x1fff) as usize] = value;
        }
    }

    fn cpu_read(&self, address: u16) -> u8 {
        if address >= 0xC000 {
            self.ines.prg_rom[(((self.ines.prg_rom_size_16k_chunks as usize) - 1) << 14) | (address & 0x3FFF) as usize]
        } else {
            self.ines.prg_rom[((self.selected_bank as usize) << 14) | (address & 0x3FFF) as usize]
        }
    }

    fn cpu_write(&mut self, address: u16, value: u8) {
        if address >= 0x8000 {
            self.selected_bank = value & 0x0F;
        }
    }
}
