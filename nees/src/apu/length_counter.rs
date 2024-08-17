use crate::reader_writer::{EasyReader, EasyWriter};

pub struct LengthCounter {
    pub value: u8,
    pub halt: bool,
}

impl LengthCounter {
    pub fn new() -> Self {
        Self {
            value: 0,
            halt: false,
        }
    }

    pub fn clock(&mut self) {
        if !self.halt && self.value > 0 {
            self.value -= 1;
        }
    }

    pub fn save(&self, writer: &mut dyn EasyWriter) -> anyhow::Result<()> {
        writer.write_u8(self.value)?;
        writer.write_bool(self.halt)?;

        Ok(())
    }

    pub fn load(&mut self, reader: &mut dyn EasyReader) -> anyhow::Result<()> {
        self.value = reader.read_u8()?;
        self.halt = reader.read_bool()?;

        Ok(())
    }
}
