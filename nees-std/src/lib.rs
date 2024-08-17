use std::io::{Read, Write};

use anyhow::anyhow;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use nees::{nes001, EasyReader, EasyWriter};

pub fn load_state(rom_path: &str, nes: &mut nes001::NES001) {
    let save_path = format!("{}.sav", rom_path);
    let mut buf_reader = MyBufReader::new(std::fs::File::open(save_path).unwrap());
    nes.load(&mut buf_reader).unwrap();
}

pub fn save_state(rom_path: &str, nes: &nes001::NES001) {
    let save_path = format!("{}.sav", rom_path);
    let mut buf_writer = MyBufWriter::new(std::fs::File::create(save_path).unwrap());
    nes.save(&mut buf_writer).unwrap();
}

struct MyBufReader<R: Read> {
    reader: R,
}

impl<R: Read> MyBufReader<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: Read> EasyReader for MyBufReader<R> {
    fn read_u8(&mut self) -> anyhow::Result<u8> {
        self.reader.read_u8().map_err(|e| anyhow!(e))
    }

    fn read_u16(&mut self) -> anyhow::Result<u16> {
        self.reader.read_u16::<LittleEndian>().map_err(|e| anyhow!(e))
    }

    fn read_i16(&mut self) -> anyhow::Result<i16> {
        self.reader.read_i16::<LittleEndian>().map_err(|e| anyhow!(e))
    }

    fn read_u32(&mut self) -> anyhow::Result<u32> {
        self.reader.read_u32::<LittleEndian>().map_err(|e| anyhow!(e))
    }

    fn read_bool(&mut self) -> anyhow::Result<bool> {
        self.read_u8().map(|v| v == 1).map_err(|e| anyhow!(e))
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> anyhow::Result<()> {
        self.reader.read_exact(buf).map_err(|e| anyhow!(e))
    }
}

struct MyBufWriter<W: Write> {
    writer: W,
}

impl<W: Write> MyBufWriter<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: Write> EasyWriter for MyBufWriter<W> {
    fn write_u8(&mut self, value: u8) -> anyhow::Result<()> {
        self.writer.write_u8(value).map_err(|e| anyhow!(e))
    }

    fn write_u16(&mut self, value: u16) -> anyhow::Result<()> {
        self.writer.write_u16::<LittleEndian>(value).map_err(|e| anyhow!(e))
    }

    fn write_i16(&mut self, value: i16) -> anyhow::Result<()> {
        self.writer.write_i16::<LittleEndian>(value).map_err(|e| anyhow!(e))
    }

    fn write_u32(&mut self, value: u32) -> anyhow::Result<()> {
        self.writer.write_u32::<LittleEndian>(value).map_err(|e| anyhow!(e))
    }

    fn write_bool(&mut self, value: bool) -> anyhow::Result<()> {
        self.writer.write_u8(if value { 1 } else { 0 }).map_err(|e| anyhow!(e))
    }

    fn write_all(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(buf).map_err(|e| anyhow!(e))
    }
}
