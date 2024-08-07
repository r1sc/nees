use std::{
    fs::File,
    io::{BufReader, Read},
};

use byteorder::ReadBytesExt;

#[allow(clippy::upper_case_acronyms)]
pub struct INES {
    pub mapper_no: u8,
    pub prg_rom_size_16k_chunks: u8,
    pub prg_rom: Vec<u8>,

    pub chr_rom_size_8kb_chunks: u8,
    pub chr_rom: Vec<u8>,
    pub is_chr_ram: bool,
    pub ppu_address_ciram_a10_shift_count: u8,
}

impl INES {
    pub fn new(path: &str) -> Self {
        let mut f = BufReader::new(File::open(path).unwrap());
        let mut nesbuf = [0_u8; 4];
        f.read_exact(&mut nesbuf)
            .expect("Failed to read NES header");
        assert!(nesbuf == [b'N', b'E', b'S', 0x1A]);

        let prg_rom_size_16k_chunks = f.read_u8().unwrap();
        let chr_rom_size_8kb_chunks = f.read_u8().unwrap();
        let is_chr_ram = chr_rom_size_8kb_chunks == 0;
        let flags6 = f.read_u8().unwrap();
        let flags7 = f.read_u8().unwrap();
        let _flags8 = f.read_u8().unwrap();
        let _flags9 = f.read_u8().unwrap();
        let _flags10 = f.read_u8().unwrap();
        
        let mut padding = [0_u8; 5];
        f.read_exact(&mut padding).unwrap();


        let mirroring = (flags6 & 1) == 1;
        let _has_wram = ((flags6 >> 1) & 1) == 1;
        let has_trainer = ((flags6 >> 2) & 1) == 1;
        let mut mapper_no = (flags7 & 0xF0) | (flags6 >> 4);

        if padding != [0, 0, 0, 0, 0] {
            mapper_no = flags6 >> 4;
        }

        if has_trainer {
            f.seek_relative(512).unwrap();
        }

        let mut prg_rom = vec![0; 16384 * (prg_rom_size_16k_chunks as usize)];
        f.read_exact(&mut prg_rom).unwrap();

        
        let mut chr_rom = vec![0; 8192 * (if is_chr_ram { 1 } else { chr_rom_size_8kb_chunks as usize })];
        if !is_chr_ram {
            f.read_exact(&mut chr_rom).unwrap();
        }

        Self {
            mapper_no,
            prg_rom_size_16k_chunks,
            prg_rom,

            chr_rom_size_8kb_chunks,
            chr_rom,
            is_chr_ram,
            ppu_address_ciram_a10_shift_count: if mirroring { 10 } else { 11 },
        }
    }
}
