use crate::reader_writer::{EasyReader, EasyWriter};

use super::{length_counter::LengthCounter, tables::LENGTH_TABLE, timer::Timer};

pub struct Triangle {
    pub enabled: bool,
    timer: Timer,
    pub length_counter: LengthCounter,
    pub linear_counter: u16,
    pub linear_counter_reload: u16,
    pub linear_counter_reload_flag: bool,
    pub current_output: u8,
    sequencer_pos: u8,
}

const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

impl Triangle {
    pub fn new() -> Self {
        Self {
            enabled: false,
            timer: Timer::new(),
            length_counter: LengthCounter::new(),
            linear_counter: 0,
            linear_counter_reload: 0,
            linear_counter_reload_flag: false,
            current_output: 0,
            sequencer_pos: 0,
        }
    }

    pub fn write_reg(&mut self, address: u8, value: u8) {
        match address {
            0 => {
                self.linear_counter_reload = (value & 0b01111111) as u16;
                self.length_counter.halt = (value & 0x80) != 0;
            }
            2 => {
                self.timer.reload = (self.timer.reload & 0x700) | value as u16;
            }
            3 => {
                if self.enabled {
                    self.length_counter.value = LENGTH_TABLE[((value & 0xF8) >> 3) as usize];
                    self.timer.reload =
                        (self.timer.reload & 0xFF) | (((value & 0b111) as u16) << 8);
                    self.timer.reload();
                    self.linear_counter_reload_flag = true;
                }
            }
            _ => {
                panic!("Invalid triangle register write");
            }
        }
    }

    pub fn tick(&mut self) {
        if self.timer.tick() && self.linear_counter > 0 && self.length_counter.value > 0 {
            self.current_output = TRIANGLE_SEQUENCE[self.sequencer_pos as usize];

            if self.sequencer_pos == 31 {
                self.sequencer_pos = 0;
            } else {
                self.sequencer_pos += 1;
            }
        }
    }

    pub fn save(&self, writer: &mut dyn EasyWriter) -> anyhow::Result<()> {
        writer.write_bool(self.enabled)?;
        self.timer.save(writer)?;
        self.length_counter.save(writer)?;
        writer.write_u16(self.linear_counter)?;
        writer.write_u16(self.linear_counter_reload)?;
        writer.write_bool(self.linear_counter_reload_flag)?;
        writer.write_u8(self.current_output)?;
        writer.write_u8(self.sequencer_pos)?;

        Ok(())
    }

    pub fn load(&mut self, reader: &mut dyn EasyReader) -> anyhow::Result<()> {
        self.enabled = reader.read_bool()?;
        self.timer.load(reader)?;
        self.length_counter.load(reader)?;
        self.linear_counter = reader.read_u16()?;
        self.linear_counter_reload = reader.read_u16()?;
        self.linear_counter_reload_flag = reader.read_bool()?;
        self.current_output = reader.read_u8()?;
        self.sequencer_pos = reader.read_u8()?;

        Ok(())
    }
}
