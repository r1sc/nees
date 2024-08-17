use crate::reader_writer::{EasyReader, EasyWriter};

use super::timer::Timer;

pub struct Envelope {
    pub start: bool,
    pub timer: Timer,
    pub decay_level: u8,
    pub constant_volume: bool,
}

impl Envelope {
    pub fn new() -> Self {
        Self {
            start: false,
            timer: Timer::new(),
            decay_level: 0,
            constant_volume: false,
        }
    }

    pub fn clock(&mut self, loop_flag: bool) {
        if self.start {
            self.start = false;
            self.decay_level = 15;
            self.timer.reload();
        } else if self.timer.tick() {
            if self.decay_level > 0 {
                self.decay_level -= 1;
            } else if loop_flag {
                self.decay_level = 15;
            }
        }
    }

    pub fn get_volume(&self) -> u8 {
        if self.constant_volume {
            self.timer.reload as u8
        } else {
            self.decay_level
        }
    }

    pub fn save(&self, writer: &mut dyn EasyWriter) -> anyhow::Result<()> {
        writer.write_bool(self.start)?;
        self.timer.save(writer)?;
        writer.write_u8(self.decay_level)?;
        writer.write_bool(self.constant_volume)?;

        Ok(())
    }

    pub fn load(&mut self, reader: &mut dyn EasyReader) -> anyhow::Result<()> {
        self.start = reader.read_bool()?;
        self.timer.load(reader)?;
        self.decay_level = reader.read_u8()?;
        self.constant_volume = reader.read_bool()?;

        Ok(())
    }
}
