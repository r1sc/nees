use nees::nes001::{self, ControllerState};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn waveout_callback(sample: i16);
}

#[wasm_bindgen]
pub fn init(rom: &[u8]) -> *mut nes001::NES001 {
    let nes = Box::new(nes001::NES001::from_rom(rom));
    Box::into_raw(nes)
}

/// .
///
/// # Panics
///
/// Panics if nes is invalid.
///
/// # Safety
///
/// .
#[wasm_bindgen]
pub unsafe fn tick(nes: *mut nes001::NES001, framebuffer: &mut [u32], player1_buttons_down: u8, player2_buttons_down: u8) {
    let nes = unsafe { nes.as_mut().unwrap() };
    let player1_controller_state = ControllerState::from_bits(player1_buttons_down);
    let player2_controller_state = ControllerState::from_bits(player2_buttons_down);
    nes.set_buttons_down(0, &player1_controller_state);
    nes.set_buttons_down(1, &player2_controller_state);
    nes.tick_frame(&mut waveout_callback, framebuffer);
}