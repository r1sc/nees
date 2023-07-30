use minifb::{Key, Window, WindowOptions};

mod nes001;
mod cartridge;
mod ines;
mod nrom;
mod ppu;
mod cpu;
mod bus;

extern "C" {
    fn nmi6502();
    fn irq6502();
    fn step6502();
    fn reset6502();
}

fn main() {

    let mut window = match Window::new("Test", 256 * 2, 240 * 2, WindowOptions::default()) {
        Ok(win) => win,
        Err(err) => {
            println!("Unable to create window {}", err);
            return;
        }
    };

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut buffer: Vec<u32> = vec![0; 256 * 240];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(&buffer, 256, 240).unwrap();
    }

    unsafe {
        reset6502();

        for i in 0..10 {
            step6502();
        }
    }
}
