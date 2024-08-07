struct Timer {
    current: u16,
    reload: u16,
}

impl Timer {
    fn new() -> Self {
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
}

struct LengthCounter {
    value: u8,
    halt: bool,
}

impl LengthCounter {
    fn new() -> Self {
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
}

struct Envelope {
    start: bool,
    timer: Timer,
    decay_level: u8,
    constant_volume: bool,
}

impl Envelope {
    fn new() -> Self {
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
}

const PULSE_DUTY_CYCLES: [u8; 4] = [0b10000000, 0b11000000, 0b11110000, 0b00111111];
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

struct Pulse {
    enabled: bool,
    timer: Timer,
    length_counter: LengthCounter,
    sequence: u8,
    sequencer_pos: u8,
    current_output: u8,
    envelope: Envelope,

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
}

struct Triangle {
    enabled: bool,
    timer: Timer,
    length_counter: LengthCounter,
    linear_counter: u16,
    linear_counter_reload: u16,
    linear_counter_reload_flag: bool,
    current_output: u8,
    sequencer_pos: u8,
}

const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

impl Triangle {
    fn new() -> Self {
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

    fn write_reg(&mut self, address: u8, value: u8) {
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

    fn tick(&mut self) {
        if self.timer.tick() && self.linear_counter > 0 && self.length_counter.value > 0 {
            self.current_output = TRIANGLE_SEQUENCE[self.sequencer_pos as usize];

            if self.sequencer_pos == 31 {
                self.sequencer_pos = 0;
            } else {
                self.sequencer_pos += 1;
            }
        }
    }
}

const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

struct Noise {
    enabled: bool,
    timer: Timer,
    length_counter: LengthCounter,
    current_output: u8,
    shift_register: u16,
    mode: bool,
    envelope: Envelope,
}

impl Noise {
    fn new() -> Self {
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

    fn tick(&mut self) {
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

    fn write_reg(&mut self, address: u8, value: u8) {
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
}

#[allow(clippy::upper_case_acronyms)]
pub struct APU {
    pulse1: Pulse,
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,

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

    pub fn write_reg(&mut self, address: u16, value: u8) {
        if (0x4000..=0x4003).contains(&address) {
            self.pulse1.write_reg((address & 0b11) as u8, value);
        } else if (0x4004..=0x4007).contains(&address) {
            self.pulse2.write_reg((address & 0b11) as u8, value);
        } else if (0x4008..=0x400B).contains(&address) {
            self.triangle.write_reg((address & 0b11) as u8, value);
        } else if (0x400C..=0x400F).contains(&address) {
            self.noise.write_reg((address & 0b11) as u8, value);
        } else if (0x4010..=0x4013).contains(&address) {
            // DMC
        } else if address == 0x4015 {
            self.pulse1.enabled = (value & 1) != 0;
            self.pulse2.enabled = (value & 2) != 0;
            self.triangle.enabled = (value & 4) != 0;
            self.noise.enabled = (value & 8) != 0;

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

    pub fn tick_triangle(&mut self) {
        if self.triangle.enabled {
            self.triangle.tick();
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
            + 2 * (self.noise.current_output as usize)];

        let sample = pulse_out + tnd_out;
        self.sample_out += (sample - self.sample_out) >> 4;

        if scanline != self.last_scanline {
            self.last_scanline = scanline;
            waveout_callback(self.sample_out);
        }

        self.cycle_counter += 1;
    }
}
