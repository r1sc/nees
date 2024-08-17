use std::{collections::VecDeque, os::raw::c_void};

use nees::nes001::{self, ControllerState};
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

    let mut player1_controller_state: ControllerState = ControllerState::new();
    let mut player2_controller_state: ControllerState = ControllerState::new();

    let dt_target = std::time::Duration::from_micros(16666);
    let mut last_time = std::time::Instant::now();
    let mut accum = std::time::Duration::ZERO;
    let mut sec_accum = std::time::Duration::ZERO;
    let one_second_duration = std::time::Duration::from_secs(1);
    let mut nes_frames = 0;

    let mut framebuffer: Vec<u32> = vec![0; 256 * 240];

    'l: loop {
        for event in sdl.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => break 'l,
                // Fill window resize event
                Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(width, height),
                    ..
                } => nees_glrenderer::resize(&gl, width, height),
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    ..
                } => player1_controller_state.set_select(true),
                Event::KeyUp {
                    keycode: Some(Keycode::Q),
                    ..
                } => player1_controller_state.set_select(false),
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    ..
                } => player1_controller_state.set_start(true),
                Event::KeyUp {
                    keycode: Some(Keycode::W),
                    ..
                } => player1_controller_state.set_start(false),
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => player1_controller_state.set_b(true),
                Event::KeyUp {
                    keycode: Some(Keycode::A),
                    ..
                } => player1_controller_state.set_b(false),
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => player1_controller_state.set_a(true),
                Event::KeyUp {
                    keycode: Some(Keycode::S),
                    ..
                } => player1_controller_state.set_a(false),
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => player1_controller_state.set_left(true),
                Event::KeyUp {
                    keycode: Some(Keycode::Left),
                    ..
                } => player1_controller_state.set_left(false),
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => player1_controller_state.set_up(true),
                Event::KeyUp {
                    keycode: Some(Keycode::Up),
                    ..
                } => player1_controller_state.set_up(false),
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => player1_controller_state.set_right(true),
                Event::KeyUp {
                    keycode: Some(Keycode::Right),
                    ..
                } => player1_controller_state.set_right(false),
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => player1_controller_state.set_down(true),
                Event::KeyUp {
                    keycode: Some(Keycode::Down),
                    ..
                } => player1_controller_state.set_down(false),
                _ => {}
            }
        }

        let now = std::time::Instant::now();
        let mut delta = now - last_time;
        last_time = now;

        if delta >= one_second_duration {
            delta = dt_target;
            accum = std::time::Duration::ZERO;
        }

        sec_accum += delta;
        accum += delta;

        if sec_accum >= std::time::Duration::from_secs(1) {
            let nes_fps = nes_frames;
            nes_frames = 0;
            sec_accum = std::time::Duration::ZERO;            

            window.set_title(format!("NES Emulator - FPS: {}", nes_fps).as_str()).unwrap();
        }

        while accum >= dt_target {
            nes.set_buttons_down(0, &player1_controller_state);
            nes.set_buttons_down(1, &player2_controller_state);
            nes.tick_frame(&mut waveout_callback, &mut framebuffer);

            accum -= dt_target;

            nes_frames += 1;
        }

        nees_glrenderer::render(&gl, &framebuffer);

        window.gl_swap_window();

        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    println!("Hello, world!");
}
