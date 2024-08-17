use crate::reader_writer::{EasyReader, EasyWriter};

use super::{envelope::Envelope, length_counter::LengthCounter, tables::LENGTH_TABLE, timer::Timer};


const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub struct Noise {
    pub enabled: bool,
    timer: Timer,
    pub length_counter: LengthCounter,
    pub current_output: u8,
    shift_register: u16,
    mode: bool,
    pub envelope: Envelope,
}

impl Noise {
    pub fn new() -> Self {
        Self {
            enabled: false,
            timer: Timer::new(),
            length_counter: LengthCounter::new(),
            current_output: 0,
            shift_register: 1,
            mode: false,
            envelope: Envelope::new(),
        }
    }

    pub fn tick(&mut self) {
        if self.timer.tick() {
            let feedback = if self.mode {
                (self.shift_register & 0x1) ^ ((self.shift_register >> 6) & 0x1)
            } else {
                (self.shift_register & 0x1) ^ ((self.shift_register >> 1) & 0x1)
            };

            self.shift_register >>= 1;
            self.shift_register = (self.shift_register & 0x3FFF) | (feedback << 14);

            if (self.shift_register & 1) == 1 || self.length_counter.value == 0 {
                self.current_output = 0;
            } else {
                self.current_output = self.envelope.get_volume();
            }
        }
    }

    pub fn write_reg(&mut self, address: u8, value: u8) {
        match address {
            0 => {
                self.length_counter.halt = (value & 0b100000) != 0;
                self.envelope.constant_volume = (value & 0b10000) != 0;
                self.envelope.timer.reload = (value & 0b1111) as u16;
            }
            2 => {
                self.timer.reload = NOISE_PERIOD_TABLE[(value & 0xF) as usize];
                self.timer.reload();
            }
            3 => {
                if self.enabled {
                    self.length_counter.value = LENGTH_TABLE[(value >> 3) as usize];
                    self.envelope.start = true;
                }
            }
            _ => {
                panic!("Invalid noise register write");
            }
        }
    }

    pub fn save(&self, writer: &mut dyn EasyWriter) -> anyhow::Result<()> {
        writer.write_bool(self.enabled)?;
        self.timer.save(writer)?;
        self.length_counter.save(writer)?;
        writer.write_u8(self.current_output)?;
        writer.write_u16(self.shift_register)?;
        writer.write_bool(self.mode)?;
        self.envelope.save(writer)?;

        Ok(())
    }

    pub fn load(&mut self, reader: &mut dyn EasyReader) -> anyhow::Result<()> {
        self.enabled = reader.read_bool()?;
        self.timer.load(reader)?;
        self.length_counter.load(reader)?;
        self.current_output = reader.read_u8()?;
        self.shift_register = reader.read_u16()?;
        self.mode = reader.read_bool()?;
        self.envelope.load(reader)?;

        Ok(())
    }
}