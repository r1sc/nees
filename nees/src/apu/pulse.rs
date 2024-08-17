use crate::reader_writer::{EasyReader, EasyWriter};

use super::{
    envelope::Envelope, length_counter::LengthCounter, tables::LENGTH_TABLE, timer::Timer,
};

const PULSE_DUTY_CYCLES: [u8; 4] = [0b10000000, 0b11000000, 0b11110000, 0b00111111];
pub struct Pulse {
    pub enabled: bool,
    timer: Timer,
    pub length_counter: LengthCounter,
    sequence: u8,
    sequencer_pos: u8,
    pub current_output: u8,
    pub envelope: Envelope,

    sweep_enabled: bool,
    sweep_divider_current: u16,
    sweep_divier_reload: u16,
    sweep_reload_flag: bool,
    sweep_negate: bool,
    sweep_shift_count: u8,
    sweep_target_period: u16,
}

impl Pulse {
    pub fn new() -> Self {
        Self {
            enabled: false,
            timer: Timer::new(),
            length_counter: LengthCounter::new(),
            sequence: 0,
            sequencer_pos: 0,
            current_output: 0,
            envelope: Envelope::new(),

            sweep_enabled: false,
            sweep_divider_current: 0,
            sweep_divier_reload: 0,
            sweep_reload_flag: false,
            sweep_negate: false,
            sweep_shift_count: 0,
            sweep_target_period: 0,
        }
    }

    pub fn calculate_sweep_target(&mut self) {
        let mut change_amount = self.timer.reload as i32 >> self.sweep_shift_count;
        if self.sweep_negate {
            change_amount = -change_amount;
        }

        let target_period = (self.timer.reload as i32 + change_amount).max(0);
        self.sweep_target_period = target_period as u16;
    }

    pub fn write_reg(&mut self, address: u8, value: u8) {
        match address {
            0 => {
                let duty_cycle_index = (value >> 6) & 0b11;
                self.sequence = PULSE_DUTY_CYCLES[duty_cycle_index as usize];
                self.length_counter.halt = (value & 0b00100000) != 0;
                self.envelope.constant_volume = (value & 0b00010000) != 0;
                self.envelope.timer.reload = (value & 0b1111) as u16;
            }
            1 => {
                self.sweep_enabled = value & 0x80 != 0;
                self.sweep_divier_reload = ((value >> 4) & 0b111) as u16;
                self.sweep_negate = (value & 0b1000) != 0;
                self.sweep_shift_count = value & 0b111;
                self.sweep_reload_flag = true;
            }
            2 => {
                self.timer.reload = (self.timer.reload & 0x700) | value as u16;
            }
            3 => {
                if self.enabled {
                    self.timer.reload =
                        (self.timer.reload & 0xFF) | (((value & 0b111) as u16) << 8);
                    self.timer.reload();
                    self.length_counter.value = LENGTH_TABLE[(value >> 3) as usize];
                    self.sequencer_pos = 0; // Reset phase
                    self.envelope.start = true;
                }
            }
            _ => {
                panic!("Invalid pulse register write");
            }
        }
    }

    pub fn tick(&mut self, _sweep_ones_complement: bool) {
        self.calculate_sweep_target();
        if self.timer.tick() {
            if self.length_counter.value > 0
                && self.timer.reload >= 8
                && self.sweep_target_period <= 0x7FF
            {
                self.current_output = if ((self.sequence >> self.sequencer_pos) & 1) != 0 {
                    self.envelope.get_volume()
                } else {
                    0
                };
            } else {
                self.current_output = 0;
            }

            if self.sequencer_pos == 0 {
                self.sequencer_pos = 7;
            } else {
                self.sequencer_pos -= 1;
            }
        }
    }

    pub fn clock_sweep_unit(&mut self) {
        if self.sweep_divider_current == 0
            && self.sweep_enabled
            && self.timer.reload >= 8
            && self.sweep_target_period <= 0x7FF
        {
            self.timer.reload = self.sweep_target_period;
        }
        if self.sweep_divider_current == 0 || self.sweep_reload_flag {
            self.sweep_divider_current = self.sweep_divier_reload;
            self.sweep_reload_flag = false;
        } else {
            self.sweep_divider_current -= 1;
        }
    }

    pub fn save(&self, writer: &mut dyn EasyWriter) -> anyhow::Result<()> {
        writer.write_bool(self.enabled)?;
        self.timer.save(writer)?;
        self.length_counter.save(writer)?;
        writer.write_u8(self.sequence)?;
        writer.write_u8(self.sequencer_pos)?;
        writer.write_u8(self.current_output)?;
        self.envelope.save(writer)?;

        writer.write_bool(self.sweep_enabled)?;
        writer.write_u16(self.sweep_divider_current)?;
        writer.write_u16(self.sweep_divier_reload)?;
        writer.write_bool(self.sweep_reload_flag)?;
        writer.write_bool(self.sweep_negate)?;
        writer.write_u8(self.sweep_shift_count)?;
        writer.write_u16(self.sweep_target_period)?;

        Ok(())
    }

    pub fn load(&mut self, reader: &mut dyn EasyReader) -> anyhow::Result<()> {
        self.enabled = reader.read_bool()?;
        self.timer.load(reader)?;
        self.length_counter.load(reader)?;
        self.sequence = reader.read_u8()?;
        self.sequencer_pos = reader.read_u8()?;
        self.current_output = reader.read_u8()?;
        self.envelope.load(reader)?;

        self.sweep_enabled = reader.read_bool()?;
        self.sweep_divider_current = reader.read_u16()?;
        self.sweep_divier_reload = reader.read_u16()?;
        self.sweep_reload_flag = reader.read_bool()?;
        self.sweep_negate = reader.read_bool()?;
        self.sweep_shift_count = reader.read_u8()?;
        self.sweep_target_period = reader.read_u16()?;

        Ok(())
    }
}
