use super::{dmc::DMC, noise::Noise, pulse::Pulse, triangle::Triangle};
use crate::{
    cartridge::CartridgeWithSaveLoad,
    reader_writer::{EasyReader, EasyWriter},
};

#[allow(clippy::upper_case_acronyms)]
pub struct APU {
    pulse1: Pulse,
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: DMC,

    cycle_counter: u32,
    five_step_mode: bool,
    interrupt_inhibit: bool,
    pub frame_interrupt_flag: bool,

    pulse_volume_lookup_table: [i16; 31],
    tnd_volume_lookup_table: [i16; 203],

    last_scanline: i16,
    sample_out: i16,
}

impl APU {
    pub fn new() -> Self {
        // Generate pulse lookup table
        let mut pulse_volume_lookup_table: [i16; 31] = [0; 31];
        for (i, value) in pulse_volume_lookup_table.iter_mut().enumerate() {
            *value = (95.52 / (8128.0 / (i as f64) + 100.0) * i16::MAX as f64) as i16;
        }

        // Generate triangle, noise and DMC lookup table
        let mut tnd_volume_lookup_table: [i16; 203] = [0; 203];
        for (i, value) in tnd_volume_lookup_table.iter_mut().enumerate() {
            *value = (163.67 / (24329.0 / (i as f64) + 100.0) * i16::MAX as f64) as i16;
        }

        Self {
            pulse1: Pulse::new(),
            pulse2: Pulse::new(),
            triangle: Triangle::new(),
            noise: Noise::new(),
            dmc: DMC::new(),

            cycle_counter: 0,
            five_step_mode: false,
            interrupt_inhibit: true,
            frame_interrupt_flag: false,
            pulse_volume_lookup_table,
            tnd_volume_lookup_table,

            last_scanline: -2,
            sample_out: 0,
        }
    }

    fn clock_linear_counters(&mut self) {
        if self.triangle.linear_counter_reload_flag {
            self.triangle.linear_counter = self.triangle.linear_counter_reload;
        } else if self.triangle.linear_counter > 0 {
            self.triangle.linear_counter -= 1;
        }

        if !self.triangle.length_counter.halt {
            self.triangle.linear_counter_reload_flag = false;
        }
    }

    fn clock_length_counters_and_sweep_units(&mut self) {
        self.pulse1.length_counter.clock();
        self.pulse2.length_counter.clock();
        self.triangle.length_counter.clock();
        self.noise.length_counter.clock();

        self.pulse1.clock_sweep_unit();
        self.pulse2.clock_sweep_unit();
    }

    fn clock_envelopes(&mut self) {
        self.pulse1.envelope.clock(self.pulse1.length_counter.halt);
        self.pulse2.envelope.clock(self.pulse2.length_counter.halt);
        self.noise.envelope.clock(self.noise.length_counter.halt);
    }

    pub fn write_reg(&mut self, address: u16, value: u8, cart: &mut dyn CartridgeWithSaveLoad) {
        if (0x4000..=0x4003).contains(&address) {
            self.pulse1.write_reg((address & 0b11) as u8, value);
        } else if (0x4004..=0x4007).contains(&address) {
            self.pulse2.write_reg((address & 0b11) as u8, value);
        } else if (0x4008..=0x400B).contains(&address) {
            self.triangle.write_reg((address & 0b11) as u8, value);
        } else if (0x400C..=0x400F).contains(&address) {
            self.noise.write_reg((address & 0b11) as u8, value);
        } else if (0x4010..=0x4013).contains(&address) {
            self.dmc.write_reg((address & 0b11) as u8, value);
        } else if address == 0x4015 {
            self.pulse1.enabled = (value & 1) != 0;
            self.pulse2.enabled = (value & 2) != 0;
            self.triangle.enabled = (value & 4) != 0;
            self.noise.enabled = (value & 8) != 0;
            self.dmc.enabled = (value & 16) != 0;

            self.dmc.interrupt_flag = false;

            if !self.pulse1.enabled {
                self.pulse1.length_counter.value = 0;
            }
            if !self.pulse2.enabled {
                self.pulse2.length_counter.value = 0;
            }
            if !self.triangle.enabled {
                self.triangle.length_counter.value = 0;
            }
            if !self.noise.enabled {
                self.noise.length_counter.value = 0;
            }
            if !self.dmc.enabled {
                self.dmc.sample_bytes_remaining = 0;
            } else {
                self.dmc.start_sample();
            }

            self.dmc.tick_memory_reader(cart);
        } else if address == 0x4017 {
            self.five_step_mode = (value & 0b10000000) != 0;
            self.interrupt_inhibit = (value & 0b01000000) != 0;

            if self.interrupt_inhibit {
                self.frame_interrupt_flag = false;
            }

            if self.five_step_mode {
                self.clock_length_counters_and_sweep_units();
            }
        }
    }
    pub fn read_reg(&mut self, address: u16) -> u8 {
        if address == 0x4015 {
            let value = ((self.interrupt_inhibit as u8) << 7)
                | ((self.frame_interrupt_flag as u8) << 6)
                | (if self.dmc.sample_bytes_remaining > 0 {
                    16
                } else {
                    0
                })
                | ((self.noise.length_counter.value > 0) as u8) << 3
                | ((self.triangle.length_counter.value > 0) as u8) << 2
                | ((self.pulse2.length_counter.value > 0) as u8) << 1
                | ((self.pulse1.length_counter.value > 0) as u8);

            self.frame_interrupt_flag = false;

            value
        } else {
            0
        }
    }

    pub fn tick_triangle(&mut self, cart: &mut dyn CartridgeWithSaveLoad) {
        if self.triangle.enabled {
            self.triangle.tick();
        }

        if self.dmc.enabled {
            self.dmc.tick(cart);
        }
    }

    pub fn tick<T: FnMut(i16)>(&mut self, scanline: i16, waveout_callback: &mut T) {
        match self.cycle_counter {
            3278 => {
                self.clock_envelopes();
                self.clock_linear_counters();
            }
            7456 => {
                self.clock_envelopes();
                self.clock_linear_counters();
                self.clock_length_counters_and_sweep_units();
            }
            11185 => {
                self.clock_envelopes();
                self.clock_linear_counters();
            }
            14914 => {
                if !self.five_step_mode {
                    if !self.interrupt_inhibit {
                        self.frame_interrupt_flag = true;
                    }
                    self.clock_envelopes();
                    self.clock_linear_counters();
                    self.clock_length_counters_and_sweep_units();
                }
            }
            14915 => {
                if !self.five_step_mode {
                    self.cycle_counter = 0;
                }
            }
            18640 => {
                if self.five_step_mode {
                    self.clock_envelopes();
                    self.clock_linear_counters();
                    self.clock_length_counters_and_sweep_units();
                }
            }
            18641 => {
                if self.five_step_mode {
                    self.cycle_counter = 0;
                }
            }
            _ => {}
        }

        if self.pulse1.enabled {
            self.pulse1.tick(true);
        }

        if self.pulse2.enabled {
            self.pulse2.tick(false);
        }

        if self.noise.enabled {
            self.noise.tick();
        }

        let pulse_out = self.pulse_volume_lookup_table
            [(self.pulse1.current_output as usize) + (self.pulse2.current_output as usize)];
        let tnd_out = self.tnd_volume_lookup_table[3 * (self.triangle.current_output as usize)
            + 2 * (self.noise.current_output as usize) + (self.dmc.output_level as usize)];

        let sample = pulse_out + tnd_out;
        self.sample_out += (sample - self.sample_out) >> 4;

        if scanline != self.last_scanline {
            self.last_scanline = scanline;
            waveout_callback(self.sample_out);
        }

        self.cycle_counter += 1;
    }

    pub fn save(&self, writer: &mut dyn EasyWriter) -> anyhow::Result<()> {
        self.pulse1.save(writer)?;
        self.pulse2.save(writer)?;
        self.triangle.save(writer)?;
        self.noise.save(writer)?;
        self.dmc.save(writer)?;

        writer.write_u32(self.cycle_counter)?;
        writer.write_bool(self.five_step_mode)?;
        writer.write_bool(self.interrupt_inhibit)?;
        writer.write_bool(self.frame_interrupt_flag)?;
        writer.write_i16(self.last_scanline)?;
        writer.write_i16(self.sample_out)?;

        Ok(())
    }

    pub fn load(&mut self, reader: &mut dyn EasyReader) -> anyhow::Result<()> {
        self.pulse1.load(reader)?;
        self.pulse2.load(reader)?;
        self.triangle.load(reader)?;
        self.noise.load(reader)?;
        self.dmc.load(reader)?;

        self.cycle_counter = reader.read_u32()?;
        self.five_step_mode = reader.read_bool()?;
        self.interrupt_inhibit = reader.read_bool()?;
        self.frame_interrupt_flag = reader.read_bool()?;
        self.last_scanline = reader.read_i16()?;
        self.sample_out = reader.read_i16()?;

        Ok(())
    }
}

impl Default for APU {
    fn default() -> Self {
        Self::new()
    }
}
