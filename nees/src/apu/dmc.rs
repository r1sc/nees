use crate::{bus::{self, Bus}, cartridge::Cartridge, nes001::NesBus, reader_writer::{EasyReader, EasyWriter}};

use super::timer::Timer;

const DMC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

pub struct DMC {
    pub enabled: bool,
    pub irq_enabled: bool,
    pub loop_flag: bool,
    pub output_level: u8,

    pub timer: Timer,

    sample_buffer_filled: bool,
    sample_buffer: u8,
    pub sample_bytes_remaining: u16,
    current_address: u16,
    sample_address: u16,
    sample_length: u16,

    shift_register: u8,
    bits_remaining: u8,
    silence: bool,
    pub interrupt_flag: bool,
}

impl DMC {
    pub fn new() -> Self {
        Self {
            enabled: false,
            irq_enabled: false,
            loop_flag: false,
            output_level: 0,

            timer: Timer::new(),

            sample_buffer_filled: false,
            sample_buffer: 0,
            sample_bytes_remaining: 0,
            current_address: 0,
            sample_address: 0,
            sample_length: 0,

            shift_register: 0,
            bits_remaining: 0,
            silence: true,
            interrupt_flag: false,
        }
    }

    pub fn start_sample(&mut self) {
        self.current_address = self.sample_address;
        self.sample_bytes_remaining = self.sample_length;
    }

    pub fn tick_memory_reader(&mut self, cart: &mut dyn Cartridge) {
        if !self.sample_buffer_filled && self.sample_bytes_remaining > 0 {
            // Sample buffer is empty, so read a new byte from memory
            self.sample_buffer = cart.cpu_read(self.current_address);
            self.current_address += 1;
            self.sample_buffer_filled = true;

            if self.current_address == 0 {
                self.current_address = 0x8000;
            }
            self.sample_bytes_remaining -= 1;
        }

        if self.sample_bytes_remaining == 0 {
            if self.loop_flag {
                self.start_sample();
            } else if self.irq_enabled {
                self.interrupt_flag = true;
            }
        }
    }

    pub fn tick(&mut self, cart: &mut dyn Cartridge) {
        let bus = self.tick_memory_reader(cart);

        if self.interrupt_flag {
            // trigger IRQ
        }

        if self.timer.tick() {
            if !self.silence {
                let b = self.shift_register & 1;
                if b == 1 && self.output_level <= 125 {
                    self.output_level += 2;
                } else if b == 0 && self.output_level >= 2 {
                    self.output_level -= 2;
                }
            }

            self.shift_register >>= 1;

            if self.bits_remaining == 0 {
                self.bits_remaining = 8;
                if !self.sample_buffer_filled {
                    self.silence = true;
                    self.output_level = 0;
                } else {
                    self.silence = false;
                    self.shift_register = self.sample_buffer;
                    self.sample_buffer_filled = false;
                }
            } else {
                self.bits_remaining -= 1;
            }
        }

        bus
    }

    pub fn write_reg(&mut self, address: u8, value: u8) {
        match address {
            0 => {
                self.irq_enabled = (value & 0x80) == 0x80;
                self.loop_flag = (value & 0x40) == 0x40;
                self.timer.reload = DMC_RATE_TABLE[(value & 0xF) as usize];
                self.timer.reload();
                if !self.irq_enabled {
                    self.interrupt_flag = false;
                }
            }
            1 => {
                self.output_level = value & 0x7F;
            }
            2 => {
                self.sample_address = 0xC000 | ((value as u16) << 6);
            }
            3 => {
                self.sample_length = ((value as u16) << 4) + 1;
            }
            _ => {
                panic!("Invalid DMC register write");
            }
        }
    }

    pub fn save(&self, mut writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        writer.write_bool(self.enabled)?;
        writer.write_bool(self.irq_enabled)?;
        writer.write_bool(self.loop_flag)?;
        writer.write_u8(self.output_level)?;

        self.timer.save(&mut writer)?;

        writer.write_bool(self.sample_buffer_filled)?;
        writer.write_u8(self.sample_buffer)?;
        writer.write_u16(self.sample_bytes_remaining)?;
        writer.write_u16(self.current_address)?;
        writer.write_u16(self.sample_address)?;
        writer.write_u16(self.sample_length)?;

        writer.write_u8(self.shift_register)?;
        writer.write_u8(self.bits_remaining)?;
        writer.write_bool(self.silence)?;
        writer.write_bool(self.interrupt_flag)?;

        Ok(())
    }

    pub fn load(&mut self, mut reader: &mut dyn std::io::Read) -> std::io::Result<()> {
        self.enabled = reader.read_bool()?;
        self.irq_enabled = reader.read_bool()?;
        self.loop_flag = reader.read_bool()?;
        self.output_level = reader.read_u8()?;

        self.timer.load(&mut reader)?;

        self.sample_buffer_filled = reader.read_bool()?;
        self.sample_buffer = reader.read_u8()?;
        self.sample_bytes_remaining = reader.read_u16()?;
        self.current_address = reader.read_u16()?;
        self.sample_address = reader.read_u16()?;
        self.sample_length = reader.read_u16()?;

        self.shift_register = reader.read_u8()?;
        self.bits_remaining = reader.read_u8()?;
        self.silence = reader.read_bool()?;
        self.interrupt_flag = reader.read_bool()?;

        Ok(())
    }
}
