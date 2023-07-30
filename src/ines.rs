use std::{
    fs::File,
    io::{BufReader, Read},
};

use byteorder::ReadBytesExt;

pub struct INES {
    pub mapper_no: u8,
    pub prg_rom_banks: Vec<Vec<u8>>,
    pub chr_rom_banks: Vec<Vec<u8>>,
    pub is_chr_ram: bool,
    pub ppu_address_ciram_a10_shift_count: u8,
}

impl INES {
    pub fn new(path: &str) -> Self {
        let mut f = BufReader::new(File::open(path).unwrap());
        let mut nesbuf = [0_u8; 4];
        f.read_exact(&mut nesbuf)
            .expect("Failed to read NES header");
        assert!(nesbuf == ['N' as u8, 'E' as u8, 'S' as u8, 0x1A]);

        let prg_rom_16kb_chunks = f.read_u8().unwrap();
        let chr_rom_8kb_chunks = f.read_u8().unwrap();
        let flags6 = f.read_u8().unwrap();
        let flags7 = f.read_u8().unwrap();
        
        let mut padding = [0_u8; 4];
        f.read_exact(&mut padding).unwrap();


        let mirroring = (flags6 & 1) == 1;
        let has_wram = ((flags6 >> 1) & 1) == 1;
        let has_trainer = ((flags6 >> 2) & 1) == 1;
        let mut mapper_no = (flags7 & 0xF0) | (flags6 >> 4);

        if padding != [0, 0, 0, 0] {
            mapper_no = flags6 >> 4;
        }

        if has_trainer {
            f.seek_relative(512).unwrap();
        }

        let prg_rom_banks: Vec<Vec<u8>> = (0..prg_rom_16kb_chunks)
            .map(|_| {
                let mut bank = vec![0; 16384 * (prg_rom_16kb_chunks as usize)];
                f.read_exact(&mut bank).unwrap();
                bank
            })
            .collect();

        let chr_rom_banks: Vec<Vec<u8>> = (0..chr_rom_8kb_chunks)
            .map(|_| {
                let mut bank = vec![0; 8192 * (prg_rom_16kb_chunks as usize)];
                f.read_exact(&mut bank).unwrap();
                bank
            })
            .collect();

        Self {
            mapper_no,
            prg_rom_banks,
            chr_rom_banks,
            is_chr_ram: chr_rom_8kb_chunks == 0,
            ppu_address_ciram_a10_shift_count: if mirroring { 10 } else { 11 },
        }
    }
}
