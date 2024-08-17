use crate::{
    bit_helpers::SubType,
    cartridge::{Cartridge, CartridgeSaveLoad, CartridgeWithSaveLoad},
    ines::INES,
    reader_writer::{EasyReader, EasyWriter},
};

#[allow(clippy::upper_case_acronyms)]
pub struct MMC2 {
    ines: INES,
    prg_rom_bank_select: u8,
    lower_fd_bank_select: u8,
    lower_fe_bank_select: u8,
    lower_latch: u8,
    upper_fd_bank_select: u8,
    upper_fe_bank_select: u8,
    upper_latch: u8,
    mirroring: u8,
}

impl MMC2 {
    pub fn new(ines: INES) -> Self {
        Self {
            ines,
            prg_rom_bank_select: 0,
            lower_fd_bank_select: 0,
            lower_fe_bank_select: 0,
            lower_latch: 0,
            upper_fd_bank_select: 0,
            upper_fe_bank_select: 0,
            upper_latch: 0,
            mirroring: 0,
        }
    }

    fn ppu_addr_to_ciram_addr(&self, ppuaddr: u16) -> u16 {
        let a10_shift_count = match self.mirroring {
            0 => 10,
            _ => 11,
        };
        (ppuaddr & 0x3ff) | (((ppuaddr >> a10_shift_count) & 1) << 10)
    }
}

const BIT_13: u16 = 1 << 13;

impl Cartridge for MMC2 {
    fn ppu_read(&mut self, address: u16, ciram: &[u8]) -> u8 {
        if (address & BIT_13) == BIT_13 {
            ciram[self.ppu_addr_to_ciram_addr(address) as usize]
        } else {
            let value = if address <= 0x0FFF {
                let chr_bank = if self.lower_latch == 0xFD {
                    self.lower_fd_bank_select
                } else {
                    self.lower_fe_bank_select
                };
                self.ines.chr_rom[(chr_bank as usize) * 0x1000 + address as usize]
            } else {
                let chr_bank = if self.upper_latch == 0xFD {
                    self.upper_fd_bank_select
                } else {
                    self.upper_fe_bank_select
                };
                self.ines.chr_rom[(chr_bank as usize) * 0x1000 + (address & 0xFFF) as usize]
            };

            if address == 0xFD8 {
                self.lower_latch = 0xFD;
            } else if address == 0xFE8 {
                self.lower_latch = 0xFE;
            } else if address >= 0x1FD8 && address <= 0x1FDF {
                self.upper_latch = 0xFD;
            } else if address >= 0x1FE8 && address <= 0x1FEF {
                self.upper_latch = 0xFE;
            }

            value
        }
    }

    fn ppu_write(&mut self, address: u16, value: u8, ciram: &mut [u8]) {
        if (address & BIT_13) == BIT_13 {
            ciram[self.ppu_addr_to_ciram_addr(address) as usize] = value;
        } else if self.ines.is_chr_ram {
            self.ines.chr_rom[address.lower_8k() as usize] = value;
        }
    }

    fn cpu_read(&self, address: u16) -> u8 {
        if address >= 0x8000 && address <= 0x9FFF {
            self.ines.prg_rom
                [8192 * self.prg_rom_bank_select as usize + (address & 0x1FFF) as usize]
        } else {
            let addr = (self.ines.prg_rom_size_16k_chunks as usize) * 0x4000
                - (0xFFFF - address) as usize
                - 1;
            self.ines.prg_rom[addr]
        }
    }

    fn cpu_write(&mut self, address: u16, value: u8) {
        if address >= 0xA000 && address <= 0xAFFF {
            self.prg_rom_bank_select = value & 0b1111;
        } else if address >= 0xB000 && address <= 0xBFFF {
            self.lower_fd_bank_select = value & 0b11111;
        } else if address >= 0xC000 && address <= 0xCFFF {
            self.lower_fe_bank_select = value & 0b11111;
        } else if address >= 0xD000 && address <= 0xDFFF {
            self.upper_fd_bank_select = value & 0b11111;
        } else if address >= 0xE000 && address <= 0xEFFF {
            self.upper_fe_bank_select = value & 0b11111;
        } else if address >= 0xF000 {
            self.mirroring = value & 1;
        }
    }

    fn scanline(&mut self) -> bool {
        false
    }
}

impl CartridgeSaveLoad for MMC2 {
    fn save(&self, writer: &mut dyn EasyWriter) -> anyhow::Result<()> {
        writer.write_u8(self.prg_rom_bank_select)?;
        writer.write_u8(self.lower_fd_bank_select)?;
        writer.write_u8(self.lower_fe_bank_select)?;
        writer.write_u8(self.lower_latch)?;
        writer.write_u8(self.upper_fd_bank_select)?;
        writer.write_u8(self.upper_fe_bank_select)?;
        writer.write_u8(self.upper_latch)?;
        writer.write_u8(self.mirroring)?;
        Ok(())
    }

    fn load(&mut self, reader: &mut dyn EasyReader) -> anyhow::Result<()> {
        self.prg_rom_bank_select = reader.read_u8()?;
        self.lower_fd_bank_select = reader.read_u8()?;
        self.lower_fe_bank_select = reader.read_u8()?;
        self.lower_latch = reader.read_u8()?;
        self.upper_fd_bank_select = reader.read_u8()?;
        self.upper_fe_bank_select = reader.read_u8()?;
        self.upper_latch = reader.read_u8()?;
        self.mirroring = reader.read_u8()?;
        Ok(())
    }
}

impl CartridgeWithSaveLoad for MMC2 {}
