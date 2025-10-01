use std::{collections::VecDeque, fs::File, io::BufWriter, os::raw::c_void};

use nees::nes001::{self, ControllerState};
use nees_std::{load_state, save_state};
use sdl2::{
    audio::{AudioCallback, AudioSpec, AudioSpecDesired},
    event::Event,
    keyboard::Keycode,
};

struct AudioBuffer {
    to_play: VecDeque<usize>,
    queue: VecDeque<usize>,
    buffers: Vec<Vec<i16>>,
    current_buffer_pos: usize,
    buffer_size: usize,
}

impl AudioBuffer {
    pub fn new(spec: AudioSpec) -> Self {
        let mut buffers = Vec::new();
        let mut queue = VecDeque::new();

        // Prepare buffers
        let num_buffers = 4;
        for i in 0..num_buffers {
            buffers.push(vec![0; spec.samples as usize]);
            queue.push_back(i);
        }
        Self {
            queue,
            buffers,
            current_buffer_pos: 0,
            buffer_size: spec.samples as usize,
            to_play: VecDeque::new(),
        }
    }

    fn queue_buffer(&mut self) {
        let element_index = self.queue.pop_front().expect("Buffer queue is empty!?");
        self.to_play.push_back(element_index);
    }

    pub fn push_sample(&mut self, sample: i16) {
        if let Some(current_buffer) = self.queue.front() {
            self.buffers[*current_buffer][self.current_buffer_pos] = sample;
            self.current_buffer_pos += 1;

            if self.current_buffer_pos >= self.buffer_size {
                self.current_buffer_pos = 0;
                self.queue_buffer();
            }
        }
    }
}

impl AudioCallback for AudioBuffer {
    type Channel = i16;

    fn callback(&mut self, data: &mut [Self::Channel]) {
        if let Some(buffer_index) = self.to_play.pop_front() {
            for (i, sample) in data.iter_mut().enumerate() {
                *sample = self.buffers[buffer_index][i];
            }
            self.queue.push_back(buffer_index);
        }
    }
}

fn main() {
    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    video.gl_attr().set_context_major_version(3);
    video.gl_attr().set_context_minor_version(2);
    video
        .gl_attr()
        .set_context_profile(sdl2::video::GLProfile::Core);

    let mut window = video
        .window("Hello, world!", 800, 600)
        .opengl()
        .resizable()
        .build()
        .unwrap();

    let _main_context = window.gl_create_context().unwrap();

    let gl = unsafe {
        glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const c_void)
    };

    // Set swap interval
    video
        .gl_set_swap_interval(sdl2::video::SwapInterval::Immediate)
        .unwrap();

    nees_glrenderer::init(&gl);

    //*** AUDIO STUFF */
    let audio = sdl.audio().unwrap();
    let desired_spec = AudioSpecDesired {
        freq: Some(15720),
        channels: Some(1),  // mono
        samples: Some(262), // default sample size
    };

    let mut device = audio
        .open_playback(None, &desired_spec, |spec| {
            // initialize the audio callback
            AudioBuffer::new(spec)
        })
        .unwrap();

    device.resume();

    let mut waveout_callback = move |sample: i16| {
        device.lock().push_sample(sample);
    };

    let rom_path = "roms/punchout.nes";
    let mut nes = nes001::NES001::from_rom(&std::fs::read(rom_path).unwrap());

    let mut player_controller_states = [ControllerState::new(), ControllerState::new()];

    let dt_target = std::time::Duration::from_micros(16666);
    let mut last_time = std::time::Instant::now();
    let mut accum = std::time::Duration::ZERO;
    let mut sec_accum = std::time::Duration::ZERO;
    let one_second_duration = std::time::Duration::from_secs(1);
    let mut nes_frames = 0;

    let mut framebuffer: Vec<u32> = vec![0; 256 * 240];

    let mut osd = nees_osd::config_menu::OSD::new();
    let mut osd_open = false;
    let mut player_select_key = [Keycode::K, Keycode::Q];
    let mut player_start_key = [Keycode::L, Keycode::W];
    let mut player_b_key = [Keycode::Comma, Keycode::A];
    let mut player_a_key = [Keycode::Period, Keycode::S];
    let mut player_up_key = [Keycode::Up, Keycode::T];
    let mut player_down_key = [Keycode::Down, Keycode::G];
    let mut player_left_key = [Keycode::Left, Keycode::F];
    let mut player_right_key = [Keycode::Right, Keycode::H];

    'l: loop {
        for event in sdl.event_pump().unwrap().poll_iter() {
            let ev = match event {
                Event::KeyDown {
                    keycode: Some(key), ..
                } => Some((key, true)),
                Event::KeyUp {
                    keycode: Some(key), ..
                } => Some((key, false)),
                _ => None,
            };

            if let Some((key, down)) = ev {
                for (i, controller_state) in &mut player_controller_states.iter_mut().enumerate() {
                    if key == player_a_key[i] {
                        controller_state.set_a(down);
                    } else if key == player_b_key[i] {
                        controller_state.set_b(down);
                    } else if key == player_down_key[i] {
                        controller_state.set_down(down);
                    } else if key == player_left_key[i] {
                        controller_state.set_left(down);
                    } else if key == player_right_key[i] {
                        controller_state.set_right(down);
                    } else if key == player_select_key[i] {
                        controller_state.set_select(down);
                    } else if key == player_start_key[i] {
                        controller_state.set_start(down);
                    } else if key == player_up_key[i] {
                        controller_state.set_up(down);
                    }
                }
            }
            match event {
                Event::Quit { .. } => break 'l,
                // Fill window resize event
                Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(width, height),
                    ..
                } => nees_glrenderer::resize(&gl, width, height),
                _ => {}
            }

            if let Event::KeyDown {
                keycode: Some(key), ..
            } = event
            {
                if key == Keycode::Escape {
                    osd_open = !osd_open;
                    osd.draw_step(&mut framebuffer);
                } else if osd_open {
                    let response = if key == Keycode::Up {
                        osd.step(nees_osd::config_menu::OSDAction::Up)
                    } else if key == Keycode::Down {
                        osd.step(nees_osd::config_menu::OSDAction::Down)
                    } else {
                        osd.step(nees_osd::config_menu::OSDAction::Ok)
                    };
                    osd.draw_step(&mut framebuffer);

                    match response {
                        nees_osd::config_menu::StepResponse::None => {}
                        nees_osd::config_menu::StepResponse::SetButtonA { which_player } => {
                            player_a_key[which_player as usize] = key
                        }
                        nees_osd::config_menu::StepResponse::SetButtonB { which_player } => {
                            player_b_key[which_player as usize] = key
                        }
                        nees_osd::config_menu::StepResponse::SetButtonSelect { which_player } => {
                            player_select_key[which_player as usize] = key
                        }
                        nees_osd::config_menu::StepResponse::SetButtonStart { which_player } => {
                            player_start_key[which_player as usize] = key
                        }
                        nees_osd::config_menu::StepResponse::SetButtonUp { which_player } => {
                            player_up_key[which_player as usize] = key
                        }
                        nees_osd::config_menu::StepResponse::SetButtonDown { which_player } => {
                            player_down_key[which_player as usize] = key
                        }
                        nees_osd::config_menu::StepResponse::SetButtonLeft { which_player } => {
                            player_left_key[which_player as usize] = key
                        }
                        nees_osd::config_menu::StepResponse::SetButtonRight { which_player } => {
                            player_right_key[which_player as usize] = key
                        }
                        nees_osd::config_menu::StepResponse::SaveState => {
                            save_state(rom_path, &nes);
                            osd_open = false;
                        }
                        nees_osd::config_menu::StepResponse::LoadState => {
                            load_state(rom_path, &mut nes);
                            osd_open = false;
                        }
                        nees_osd::config_menu::StepResponse::HorizontalAdjustment(_) => todo!(),
                    }
                }
            }
        }

        let now = std::time::Instant::now();
        let mut delta = now - last_time;
        last_time = now;

        if delta >= one_second_duration {
            delta = dt_target;
            accum = std::time::Duration::ZERO;
        }

        if !osd_open {
            sec_accum += delta;
            accum += delta;

            if sec_accum >= std::time::Duration::from_secs(1) {
                let nes_fps = nes_frames;
                nes_frames = 0;
                sec_accum = std::time::Duration::ZERO;

                window
                    .set_title(format!("NES Emulator - FPS: {}", nes_fps).as_str())
                    .unwrap();
            }

            while accum >= dt_target {
                nes.set_buttons_down(0, &player_controller_states[0]);
                nes.set_buttons_down(1, &player_controller_states[1]);
                nes.tick_frame(&mut waveout_callback, &mut framebuffer);

                accum -= dt_target;

                nes_frames += 1;
            }
        }

        nees_glrenderer::render(&gl, &framebuffer);

        window.gl_swap_window();

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
