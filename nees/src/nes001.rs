use bitfield_struct::bitfield;

use crate::{
    apu::APU,
    bus::Bus,
    cartridge::Cartridge,
    cpu,
    ines::INES,
    mappers,
    ppu::PPU,
    reader_writer::{EasyReader, EasyWriter},
};

pub struct NesBus {
    cpu_ram: Vec<u8>,
    cart: Box<dyn Cartridge + Sync + Send>,
    ppu: PPU,
    pub apu: APU,
    controller_status: [u8; 2],
    pub buttons_down: [u8; 2],
    cpu_timer: u32,
    apu_timer: u32,
}
impl NesBus {
    pub fn new(cart: Box<dyn Cartridge + Sync + Send>) -> Self {
        Self {
            cart,
            cpu_ram: vec![0; 2048],
            ppu: PPU::new(),
            apu: APU::new(),
            controller_status: [0, 0],
            buttons_down: [0, 0],
            cpu_timer: 0,
            apu_timer: 0,
        }
    }

    fn save(&self, mut writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        writer.write_all(&self.cpu_ram)?;
        self.cart.save(writer)?;
        self.ppu.save(writer)?;
        //self.apu.save(writer)?;
        writer.write_u32(self.cpu_timer)?;
        writer.write_u32(self.apu_timer)?;

        Ok(())
    }

    fn load(&mut self, mut reader: &mut dyn std::io::Read) -> std::io::Result<()> {
        reader.read_exact(&mut self.cpu_ram)?;
        self.cart.load(reader)?;
        self.ppu.load(reader)?;
        //self.apu.load(reader)?;
        self.cpu_timer = reader.read_u32()?;
        self.apu_timer = reader.read_u32()?;

        Ok(())
    }
}

impl Bus for NesBus {
    fn cpu_read(&mut self, address: u16) -> u8 {
        if address == 0x4016 || address == 0x4017 {
            let controller_id = address & 1;
            let value = self.controller_status[controller_id as usize] & 1;
            self.controller_status[controller_id as usize] >>= 1;
            value
        } else if (address >= 0x4000 && address <= 0x4013) || address == 0x4015 || address == 0x4017
        {
            // APU
            self.apu.read_reg(address)
        } else if address >= 0x4000 {
            // Cart
            self.cart.cpu_read(address)
        } else if address >= 0x2000 {
            // PPU
            self.ppu
                .cpu_ppu_bus_read((address & 7) as u8, &mut *self.cart)
        } else {
            // CPU
            self.cpu_ram[(address & 0x7ff) as usize]
        }
    }

    fn cpu_write(&mut self, address: u16, value: u8) {
        if address == 0x4014 {
            // DMA
            let page = (value as u16) << 8;
            for i in 0..256 {
                let data = self.cpu_read(page | i);
                self.ppu.cpu_ppu_bus_write(4, data, &mut *self.cart);
            }
            self.cpu_timer += 513;
        } else if address == 0x4016 {
            self.controller_status[0] = self.buttons_down[0];
            self.controller_status[1] = self.buttons_down[1];
        } else if (address >= 0x4000 && address <= 0x4013) || address == 0x4015 || address == 0x4017
        {
            self.apu.write_reg(address, value);
        } else if address >= 0x4000 {
            // Cart
            self.cart.cpu_write(address, value);
        } else if address >= 0x2000 {
            // PPU
            self.ppu
                .cpu_ppu_bus_write((address & 7) as u8, value, &mut *self.cart);
        } else {
            // CPU
            self.cpu_ram[(address & 0x7ff) as usize] = value;
        }
    }
}

#[bitfield(u8)]
pub struct ControllerState {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

pub struct NES001 {
    cpu: cpu::MOS6502<NesBus>,
    bus: NesBus,
    pub framebuffer: Vec<u32>,
}

impl NES001 {
    pub fn from_rom(rom: &[u8]) -> Self {
        let cart = mappers::load_cart(INES::new(rom));
        Self::new(cart)
    }

    fn new(cart: Box<dyn Cartridge + Sync + Send>) -> Self {
        let mut bus = NesBus::new(cart);
        let mut cpu = cpu::MOS6502::new();
        cpu.reset(&mut bus);

        Self {
            bus,
            cpu,
            framebuffer: vec![0; 256 * 240],
        }
    }

    pub fn tick_frame<T: FnMut(i16)>(&mut self, waveout_callback: &mut T) {
        for scanline in -1..=260 {
            for dot in 0..=340 {
                if self
                    .bus
                    .ppu
                    .tick(scanline, dot, &mut self.framebuffer, &mut *self.bus.cart)
                {
                    self.cpu.nmi6502(&mut self.bus);
                }

                if self.bus.cpu_timer == 0 {
                    self.cpu.step(&mut self.bus);
                    self.bus.cpu_timer = self.cpu.clockticks * 3;
                } else {
                    self.bus.cpu_timer -= 1;
                }

                if self.bus.apu_timer == 2 {
                    self.bus.apu.tick_triangle();
                }

                if self.bus.apu_timer == 5 {
                    self.bus.apu.tick_triangle();
                    self.bus.apu.tick(scanline as i16, waveout_callback);
                    self.bus.apu_timer = 0;
                } else {
                    self.bus.apu_timer += 1;
                }

                if self.bus.apu.frame_interrupt_flag {
                    self.cpu.irq6502(&mut self.bus);
                }
            }
            if scanline > -1
                && scanline <= 239
                && self.bus.ppu.is_rending_enabled()
                && self.bus.cart.scanline()
            {
                self.cpu.irq6502(&mut self.bus);
            }
        }
    }

    pub fn set_buttons_down(&mut self, controller: u8, state: &ControllerState) {
        self.bus.buttons_down[controller as usize] = state.0;
    }

    pub fn save(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        self.cpu.save(writer)?;
        self.bus.save(writer)?;

        Ok(())
    }

    pub fn load(&mut self, reader: &mut dyn std::io::Read) -> std::io::Result<()> {
        self.cpu.load(reader)?;
        self.bus.load(reader)?;

        Ok(())
    }
}
