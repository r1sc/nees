use crate::{
    bit_helpers::{SubType, MASK_16K},
    cartridge::Cartridge,
    ines::INES,
    reader_writer::{EasyReader, EasyWriter},
};

#[allow(clippy::upper_case_acronyms)]
pub struct MMC3 {
    ines: INES,
    ram: [u8; 1024 * 8], // 8KB,
    mirroring: u8,
    bank_to_update: u8,
    prg_rom_bank_mode: bool,
    chr_a12_inversion: bool,

    chr_banks: [u8; 8],
    prg_banks: [u8; 4],
    registers: [u8; 8],
    irq_latch: u8,
    irq_counter: u16,
    irq_enabled: bool,
    irq_reload: bool,
}

impl MMC3 {
    pub fn new(ines: INES) -> Self {
        let prg_rom_size_16k_chunks = ines.prg_rom_size_16k_chunks;

        let prg_banks = [
            0,
            1,
            ines.prg_rom_size_16k_chunks * 2 - 2,
            ines.prg_rom_size_16k_chunks * 2 - 1,
        ];

        Self {
            ines,
            mirroring: 0,
            bank_to_update: 0,
            prg_rom_bank_mode: false,
            chr_a12_inversion: false,
            registers: [0; 8],
            chr_banks: [0; 8],
            prg_banks,

            irq_latch: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_reload: false,

            ram: [0; 1024 * 8],
        }
    }

    fn ppu_addr_to_ciram_addr(&self, ppuaddr: u16) -> u16 {
        let mut a10_shift_count = match self.mirroring {
            0 => 10,
            _ => 11,
        };
        (ppuaddr & 0x3ff) | (((ppuaddr >> a10_shift_count) & 1) << 10)
    }
}

const BIT_13: u16 = 1 << 13;

impl Cartridge for MMC3 {
    fn ppu_read(&mut self, address: u16, ciram: &[u8]) -> u8 {
        if (address & BIT_13) == BIT_13 {
            ciram[self.ppu_addr_to_ciram_addr(address) as usize]
        } else {
            let bank = match address {
                0x0000..=0x3FF => self.chr_banks[0],
                0x0400..=0x7FF => self.chr_banks[1],
                0x0800..=0xBFF => self.chr_banks[2],
                0x0C00..=0xFFF => self.chr_banks[3],
                0x1000..=0x13FF => self.chr_banks[4],
                0x1400..=0x17FF => self.chr_banks[5],
                0x1800..=0x1BFF => self.chr_banks[6],
                0x1C00..=0x1FFF => self.chr_banks[7],
                _ => 0,
            };
            self.ines.chr_rom[(bank as usize) * 1024 + (address & 0x3FF) as usize]
        }
    }

    fn ppu_write(&mut self, address: u16, value: u8, ciram: &mut [u8]) {
        if (address & BIT_13) == BIT_13 {
            ciram[self.ppu_addr_to_ciram_addr(address) as usize] = value;
        } else if self.ines.is_chr_ram && address < 0x2000 {
            self.ines.chr_rom[address as usize] = value;
        }
    }

    fn cpu_read(&self, address: u16) -> u8 {
        if address >= 0x6000 && address <= 0x7FFF {
            self.ram[(address & 0x1FFF) as usize]
        } else {
            let bank = match address {
                0x8000..=0x9FFF => self.prg_banks[0],
                0xA000..=0xBFFF => self.prg_banks[1],
                0xC000..=0xDFFF => self.prg_banks[2],
                0xE000..=0xFFFF => self.prg_banks[3],
                _ => 0,
            };

            self.ines.prg_rom[(bank as usize) * 8192 + (address & 0x1FFF) as usize]
        }
    }

    fn cpu_write(&mut self, address: u16, value: u8) {
        let address_even = address & 1 == 0;

        if address >= 0x6000 && address <= 0x7FFF {
            self.ram[(address & 0x1FFF) as usize] = value;
        } else if address >= 0x8000 && address <= 0x9FFF {
            if address_even {
                self.bank_to_update = value & 0b111;
                self.prg_rom_bank_mode = (value & 0x40) == 0x40;
                self.chr_a12_inversion = (value & 0x80) == 0x80;
            } else {
                self.registers[self.bank_to_update as usize] = value;

                if self.chr_a12_inversion {
                    self.chr_banks[0] = self.registers[0];
                    self.chr_banks[0] = self.registers[2];
                    self.chr_banks[1] = self.registers[3];
                    self.chr_banks[2] = self.registers[4];
                    self.chr_banks[3] = self.registers[5];
                    self.chr_banks[4] = self.registers[0] & 0xFE;
                    self.chr_banks[5] = (self.registers[0] & 0xFE) + 1;
                    self.chr_banks[6] = self.registers[1] & 0xFE;
                    self.chr_banks[7] = (self.registers[1] & 0xFE) + 1;
                } else {
                    self.chr_banks[0] = self.registers[0] & 0xFE;
                    self.chr_banks[1] = (self.registers[0] & 0xFE) + 1;
                    self.chr_banks[2] = (self.registers[1] & 0xFE);
                    self.chr_banks[3] = (self.registers[1] & 0xFE) + 1;
                    self.chr_banks[4] = self.registers[2];
                    self.chr_banks[5] = self.registers[3];
                    self.chr_banks[6] = self.registers[4];
                    self.chr_banks[7] = self.registers[5];
                }

                let num_8k_prg_banks = self.ines.prg_rom_size_16k_chunks * 2;
                if self.prg_rom_bank_mode {
                    self.prg_banks[0] = num_8k_prg_banks - 2;
                    self.prg_banks[2] = self.registers[6] & 0x3F;
                } else {
                    self.prg_banks[0] = self.registers[6] & 0x3F;
                    self.prg_banks[2] = num_8k_prg_banks - 2;
                }
                self.prg_banks[1] = self.registers[7] & 0x3F;
            }
        } else if address >= 0xA000 && address <= 0xBFFF {
            if address_even {
                self.mirroring = value & 1;
            } else {
                // Skip PRG RAM protect register
            }
        } else if address >= 0xC000 && address <= 0xDFFF {
            if address_even {
                self.irq_latch = value;
            } else {
                self.irq_counter = 0;
                self.irq_reload = true;
            }
        } else if address >= 0xE000 {
            self.irq_enabled = !address_even;
        }
    }

    fn scanline(&mut self) -> bool {
        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = self.irq_latch as u16;
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }

        if self.irq_counter == 0 && self.irq_enabled {
            // Trigger IRQ
            true
        } else {
            false
        }
    }

    fn save(&self, mut writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        writer.write_all(&self.ram)?;
        writer.write_u8(self.mirroring)?;
        writer.write_u8(self.bank_to_update)?;
        writer.write_bool(self.prg_rom_bank_mode)?;
        writer.write_bool(self.chr_a12_inversion)?;

        writer.write_all(&self.chr_banks)?;
        writer.write_all(&self.prg_banks)?;
        writer.write_all(&self.registers)?;
        writer.write_u8(self.irq_latch)?;
        writer.write_u16(self.irq_counter)?;
        writer.write_bool(self.irq_enabled)?;
        writer.write_bool(self.irq_reload)?;

        Ok(())
    }

    fn load(&mut self, mut reader: &mut dyn std::io::Read) -> std::io::Result<()> {
        reader.read_exact(&mut self.ram)?;
        self.mirroring = reader.read_u8()?;
        self.bank_to_update = reader.read_u8()?;
        self.prg_rom_bank_mode = reader.read_bool()?;
        self.chr_a12_inversion = reader.read_bool()?;
        
        reader.read_exact(&mut self.chr_banks)?;
        reader.read_exact(&mut self.prg_banks)?;
        reader.read_exact(&mut self.registers)?;
        self.irq_latch = reader.read_u8()?;
        self.irq_counter = reader.read_u16()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_reload = reader.read_bool()?;

        Ok(())
    }
}
