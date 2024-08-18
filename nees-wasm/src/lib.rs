use nees::nes001::{self, ControllerState};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn waveout_callback(sample: i16);
}

#[wasm_bindgen]
pub fn get_framebuffer_ptr() -> *mut u32 {
    let mut framebuffer = vec![15; 256 * 240];
    let ptr = framebuffer.as_mut_ptr();
    std::mem::forget(framebuffer);
    ptr
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
/// Panics if nes or framebuffer_ptr is invalid.
///
/// # Safety
///
/// .
#[wasm_bindgen]
pub unsafe fn tick(nes: *mut nes001::NES001, framebuffer_ptr: *mut u32, player1_buttons_down: u8, player2_buttons_down: u8) {
    let nes = unsafe { nes.as_mut().unwrap() };
    let player1_controller_state = ControllerState::from_bits(player1_buttons_down);
    let player2_controller_state = ControllerState::from_bits(player2_buttons_down);
    nes.set_buttons_down(0, &player1_controller_state);
    nes.set_buttons_down(1, &player2_controller_state);

    let framebuffer = unsafe { std::slice::from_raw_parts_mut(framebuffer_ptr, 256 * 240) };
    nes.tick_frame(&mut waveout_callback, framebuffer);
}