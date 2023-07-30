use crate::{bus::Bus, cartridge::Cartridge, ppu::PPU, cpu::MOS6502};

struct NESState {
}

pub struct NES001 {
    pub framebuffer: [u32; 256 * 240],
    cartridge: Option<Box<dyn Cartridge>>,
    ciram: [u8; 256],
    cpu_timer: usize,
    ppu: PPU,
    cpu: MOS6502<Self>
}

#[no_mangle]
extern "C" fn read6502(address: u16) -> u8 {
    println!("Read {:x}", address);
    0xEA
}

#[no_mangle]
extern "C" fn write6502(address: u16, value: u8) {
    println!("Write {:x} = {:x}", address, value);
}

impl NES001 {
    pub fn new() -> Self {
        Self {
            framebuffer: [0; 256 * 240],
            ciram: [0; 256],
            cartridge: None,
            ppu: PPU::new(),
            cpu_timer: 0,
            cpu: MOS6502::new()
        }
    }

    pub fn tick_frame(&mut self) {
        if let Some(cart) = &self.cartridge {
            self.ppu.tick(&mut self.framebuffer, cart);
        }
    }
}

impl Bus for NES001 {
    fn cpu_read(&self, address: u16) -> u8 {
        todo!()
    }

    fn cpu_write(&mut self, address: u16, value: u8) {
        if address >= 0x2000 {
            if let Some(cart) = &mut self.cartridge {
                self.ppu.cpu_ppu_bus_write(address, value, cart);
            }
        }
    }

    fn ppu_read(&self, address: u16) -> u8 {
        if let Some(cart) = &self.cartridge {
            cart.ppu_read(address, &self.ciram)
        } else {
            0
        }
    }

    fn ppu_write(&mut self, address: u16, value: u8) {
        if let Some(cart) = &mut self.cartridge {
            cart.ppu_write(address, value, &mut self.ciram);
        }
    }
}
