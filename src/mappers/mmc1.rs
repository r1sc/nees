use crate::{bit_helpers::SubType, cartridge::Cartridge, ines::INES};

#[allow(clippy::upper_case_acronyms)]
pub struct MMC1 {
    ines: INES,
    control_reg: u8,
    sr: u8,
    shift_count: u8,

    chr_bank_4_lo: u8,
    chr_bank_4_hi: u8,
    chr_bank_8: u8,

    prg_bank_lo: u8,
    prg_bank_hi: u8,
    prg_bank_32: u8,

    mirroring: u8,

    ram: [u8; 1024 * 32], // 32KB
}

impl MMC1 {
    pub fn new(ines: INES) -> Self {
        let prg_rom_size_16k_chunks = ines.prg_rom_size_16k_chunks;
        Self {
            ines,
            control_reg: 0x1c,
            sr: 0,
            shift_count: 0,

            mirroring: 3,
            chr_bank_4_lo: 0,
            chr_bank_4_hi: 0,
            chr_bank_8: 0,

            prg_bank_lo: 0,
            prg_bank_hi: prg_rom_size_16k_chunks - 1,
            prg_bank_32: 0,

            ram: [0; 1024 * 32],
        }
    }

    fn ppu_addr_to_ciram_addr(&self, ppuaddr: u16) -> u16 {
        let mut a10_shift_count = self.ines.ppu_address_ciram_a10_shift_count;
        match self.mirroring {
            0 => {
                // one-screen, lower bank
            }
            1 => {
                // one-screen, upper bank
            }
            2 => a10_shift_count = 10,
            3 => {
                // horizontal
                a10_shift_count = 11;
            }
            _ => {}
        }
        (ppuaddr & 0x3ff) | (((ppuaddr >> a10_shift_count) & 1) << 10)
    }
}

const BIT_13: u16 = 1 << 13;

impl Cartridge for MMC1 {
    fn ppu_read(&self, address: u16, ciram: &[u8]) -> u8 {
        if (address & BIT_13) == BIT_13 {
            ciram[self.ppu_addr_to_ciram_addr(address) as usize]
        } else if self.ines.is_chr_ram {
            self.ines.chr_rom[address as usize]
        } else if (self.control_reg & 0b10000) != 0 {
            // switch two separate 4 KB banks
            if address < 0x1000 {
                self.ines.chr_rom
                    [(self.chr_bank_4_lo as usize) * 0x1000 + address.lower_4k() as usize]
            } else {
                self.ines.chr_rom
                    [(self.chr_bank_4_hi as usize) * 0x1000 + address.lower_4k() as usize]
            }
        } else {
            // switch 8 KB at a time
            self.ines.chr_rom[(self.chr_bank_8 % self.ines.chr_rom_size_8kb_chunks) as usize
                * 0x2000
                + (address & 0x1FFF) as usize]
        }
    }

    fn ppu_write(&mut self, address: u16, value: u8, ciram: &mut [u8]) {
        if (address & BIT_13) == BIT_13 {
            ciram[self.ppu_addr_to_ciram_addr(address) as usize] = value;
        } else if self.ines.is_chr_ram {
            self.ines.chr_rom[address as usize] = value;
        }
    }

    fn cpu_read(&self, address: u16) -> u8 {
        if address >= 0x6000 && address <= 0x7FFF {
            self.ram[address as usize]
        } else if address >= 0x8000 {
            if (self.control_reg & 0b01000) != 0 {
                if address >= 0xC000 {
                    self.ines.prg_rom
                        [(self.prg_bank_hi as usize) * 0x4000 + address.lower_16k() as usize]
                } else {
                    self.ines.prg_rom
                        [(self.prg_bank_lo as usize) * 0x4000 + address.lower_16k() as usize]
                }
            } else {
                self.ines.prg_rom
                    [(self.prg_bank_32 as usize) * 0x8000 + address.lower_32k() as usize]
            }
        } else {
            0
        }
    }

    fn cpu_write(&mut self, address: u16, value: u8) {
        if address >= 0x6000 && address <= 0x7FFF {
            self.ram[address as usize] = value;
        } else if address >= 0x8000 {
            if (value & 0x80) != 0 {
                self.sr = 0;
                self.shift_count = 0;
            } else {
                self.sr = (self.sr >> 1) | ((value & 1) << 4);
                self.shift_count += 1;
                if self.shift_count == 5 {
                    let reg = (address >> 13) & 0b11;
                    match reg {
                        0 => {
                            // Control
                            self.mirroring = self.sr & 0b11;
                            self.control_reg = self.sr & 0x1f;
                        }
                        1 => {
                            // CHR bank 0
                            if (self.control_reg & 0b10000) != 0 {
                                self.chr_bank_4_lo = self.sr & 0x1f;
                            } else {
                                self.chr_bank_8 = self.sr & 0x1e;
                            }
                        }
                        2 => {
                            if (self.control_reg & 0b10000) != 0 {
                                self.chr_bank_4_hi = self.sr & 0x1F;
                            }
                        }
                        3 => {
                            let prg_mode = (self.control_reg >> 2) & 0x03;
                            match prg_mode {
                                0 | 1 => {
                                    // switch 32 KB at $8000
                                    self.prg_bank_32 = (self.sr & 0x0e) >> 1;
                                }
                                2 => {
                                    // fix first bank at $8000 and switch 16 KB bank at $C000
                                    self.prg_bank_lo = 0;
                                    self.prg_bank_hi = self.sr & 0x0f;
                                }
                                3 => {
                                    // fix last bank at $C000 and switch 16 KB bank at $8000)
                                    self.prg_bank_lo = self.sr & 0x0f;
                                    self.prg_bank_hi = self.ines.prg_rom_size_16k_chunks - 1;
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                    self.sr = 0;
                    self.shift_count = 0;
                }
            }
        }
    }
}
