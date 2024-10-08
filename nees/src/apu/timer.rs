use crate::reader_writer::{EasyReader, EasyWriter};

pub struct Timer {
    current: u16,
    pub reload: u16,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            current: 0,
            reload: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.current == 0 {
            self.current = self.reload;
            true
        } else {
            self.current -= 1;
            false
        }
    }

    pub fn reload(&mut self) {
        self.current = self.reload;
    }

    pub fn save(&self, writer: &mut dyn EasyWriter) -> anyhow::Result<()> {
        writer.write_u16(self.current)?;
        writer.write_u16(self.reload)?;

        Ok(())
    }

    pub fn load(&mut self, reader: &mut dyn EasyReader) -> anyhow::Result<()> {
        self.current = reader.read_u16()?;
        self.reload = reader.read_u16()?;

        Ok(())
    }
}