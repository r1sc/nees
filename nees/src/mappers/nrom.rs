use crate::{
    bit_helpers::{SubType, BIT_13, MASK_16K, MASK_32K},
    cartridge::Cartridge,
    ines::INES,
};

#[allow(clippy::upper_case_acronyms)]
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

impl Cartridge for NROM {
    fn ppu_read(&mut self, address: u16, ciram: &[u8]) -> u8 {
        if (address & BIT_13) == BIT_13 {
            ciram[self.nrom_ppu_addr_to_ciram_addr(address) as usize]
        } else {
            self.ines.chr_rom[address.lower_8k() as usize]
        }
    }

    fn ppu_write(&mut self, address: u16, value: u8, ciram: &mut [u8]) {
        if (address & BIT_13) == BIT_13 {
            ciram[self.nrom_ppu_addr_to_ciram_addr(address) as usize] = value;
        }
    }

    fn cpu_read(&self, address: u16) -> u8 {
        self.ines.prg_rom[(address
            & (if self.ines.prg_rom_size_16k_chunks == 1 {
                MASK_16K
            } else {
                MASK_32K
            })) as usize]
    }

    fn cpu_write(&mut self, _address: u16, _value: u8) {
        // Do nothing
    }

    fn scanline(&mut self) -> bool {
        false
    }
    
    fn save(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        Ok(())
    }
    
    fn load(&mut self, reader: &mut dyn std::io::Read) -> std::io::Result<()> {
        Ok(())
    }
}
