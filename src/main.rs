use ines::INES;
use minifb::{Key, Window, WindowOptions};
use nes001::ControllerState;

mod nes001;
mod cartridge;
mod ines;
mod mappers;
mod ppu;
mod cpu;
mod bus;

fn main() {
    let ines = INES::new("roms/chip.nes");
    let cart = mappers::load_cart(ines);
    let mut nes = nes001::NES001::new(cart);
    
    let mut window = match Window::new("Test", 256 * 2, 240 * 2, WindowOptions::default()) {
        Ok(win) => win,
        Err(err) => {
            println!("Unable to create window {}", err);
            return;
        }
    };
    
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut controller_state: ControllerState = ControllerState::new();

    let mut frames = 0;
    let mut fps_timer = std::time::Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = std::time::Instant::now();

        frames += 1;

        if now - fps_timer >= std::time::Duration::from_secs(1) {
            window.set_title(format!("NES Emulator - FPS: {}", frames).as_str());
            frames = 0;
            fps_timer = now;
        }        

        controller_state.set_right(window.is_key_down(Key::Right));
        controller_state.set_left(window.is_key_down(Key::Left));
        controller_state.set_down(window.is_key_down(Key::Down));
        controller_state.set_up(window.is_key_down(Key::Up));
        controller_state.set_start(window.is_key_down(Key::W));
        controller_state.set_select(window.is_key_down(Key::Q));
        controller_state.set_b(window.is_key_down(Key::A));
        controller_state.set_a(window.is_key_down(Key::S));
        
        nes.set_buttons_down(0, &controller_state);
        nes.tick_frame();
        window.update_with_buffer(&nes.fb, 256, 240).unwrap(); 
    }

}
