use core::panic;

use nees::nes001::{self, ControllerState};
use nees_osd::config_menu::OSDAction;
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

pub struct State {
    nes: nes001::NES001,
    osd: nees_osd::config_menu::OSD,
}

#[wasm_bindgen]
pub fn init(rom: &[u8]) -> *mut State {
    let state = Box::new(State {
        nes: nes001::NES001::from_rom(rom),
        osd: nees_osd::config_menu::OSD::new(),
    });
    Box::into_raw(state)
}

#[wasm_bindgen]
pub unsafe fn tick(
    state: *mut State,
    framebuffer_ptr: *mut u32,
    player1_buttons_down: u8,
    player2_buttons_down: u8,
) {
    let state = unsafe { state.as_mut().unwrap() };
    let player1_controller_state = ControllerState::from_bits(player1_buttons_down);
    let player2_controller_state = ControllerState::from_bits(player2_buttons_down);
    state.nes.set_buttons_down(0, &player1_controller_state);
    state.nes.set_buttons_down(1, &player2_controller_state);

    let framebuffer = unsafe { std::slice::from_raw_parts_mut(framebuffer_ptr, 256 * 240) };
    state.nes.tick_frame(&mut waveout_callback, framebuffer);
}

#[wasm_bindgen]
pub struct StepResponse {
    pub action: u8,
    pub value: i16,
}

#[wasm_bindgen]
pub unsafe fn step_osd(state: *mut State, action: u8) -> StepResponse {
    let state = unsafe { state.as_mut().unwrap() };
    match state.osd.step(match action {
        0 => OSDAction::Up,
        1 => OSDAction::Down,
        2 => OSDAction::Ok,
        _ => panic!("Invalid"),
    }) {
        nees_osd::config_menu::StepResponse::None => StepResponse {
            action: 0,
            value: 0,
        },
        nees_osd::config_menu::StepResponse::SetButtonB { which_player } => StepResponse {
            action: 1,
            value: which_player as i16,
        },
        nees_osd::config_menu::StepResponse::SetButtonA { which_player } => StepResponse {
            action: 2,
            value: which_player as i16,
        },
        nees_osd::config_menu::StepResponse::SetButtonSelect { which_player } => StepResponse {
            action: 3,
            value: which_player as i16,
        },
        nees_osd::config_menu::StepResponse::SetButtonStart { which_player } => StepResponse {
            action: 4,
            value: which_player as i16,
        },
        nees_osd::config_menu::StepResponse::SetButtonUp { which_player } => StepResponse {
            action: 5,
            value: which_player as i16,
        },
        nees_osd::config_menu::StepResponse::SetButtonDown { which_player } => StepResponse {
            action: 6,
            value: which_player as i16,
        },
        nees_osd::config_menu::StepResponse::SetButtonLeft { which_player } => StepResponse {
            action: 7,
            value: which_player as i16,
        },
        nees_osd::config_menu::StepResponse::SetButtonRight { which_player } => StepResponse {
            action: 8,
            value: which_player as i16,
        },
        nees_osd::config_menu::StepResponse::SaveState => StepResponse {
            action: 9,
            value: 0,
        },
        nees_osd::config_menu::StepResponse::LoadState => StepResponse {
            action: 10,
            value: 0,
        },
        nees_osd::config_menu::StepResponse::HorizontalAdjustment(value) => {
            StepResponse { action: 11, value }
        }
    }
}

#[wasm_bindgen]
pub unsafe fn draw_osd(state: *mut State, framebuffer_ptr: *mut u32) {
    let state = unsafe { state.as_mut().unwrap() };
    let framebuffer = unsafe { std::slice::from_raw_parts_mut(framebuffer_ptr, 256 * 240) };
    state.osd.draw_step(framebuffer);
}

#[wasm_bindgen]
pub unsafe fn save_state(state: *const State) -> Vec<u8> {
    let state = unsafe { state.as_ref().unwrap() };
    let mut writer = SaveThing::new();
    nees_std::save_state_buffer(&state.nes, &mut writer);
    writer.0
}


#[wasm_bindgen]
pub unsafe fn load_state(state: *mut State, buffer: &[u8]) {
    let state = unsafe { state.as_mut().unwrap() };
    let mut reader = SaveThing::from_buffer(buffer);
    nees_std::load_state_buffer(&mut state.nes, &mut reader);
}


struct SaveThing(pub Vec<u8>);
impl SaveThing {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn from_buffer(buffer: &[u8]) -> Self {
        Self(buffer.to_vec())
    }
}
impl std::io::Write for SaveThing {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl std::io::Read for SaveThing {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = std::cmp::min(buf.len(), self.0.len());
        buf[..len].copy_from_slice(&self.0[..len]);
        self.0 = self.0[len..].to_vec();
        Ok(len)
    }
}