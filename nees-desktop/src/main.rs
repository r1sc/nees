// #![windows_subsystem = "windows"]

use std::io::BufWriter;

use glow::HasContext;
use nees::nes001;
use nes001::ControllerState;

mod platform;
mod gamepad;


fn load_shader(gl: &glow::Context, shader_type: u32, source: &str) -> glow::Shader {
    unsafe {
        let shader = gl.create_shader(shader_type).expect("Cannot create shader");

        let source = format!(
            "#version 300 es\n{}\n{}",
            if shader_type == glow::VERTEX_SHADER {
                "#define VS"
            } else {
                "#define FS"
            },
            source
        );

        gl.shader_source(shader, &source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            panic!("{}", gl.get_shader_info_log(shader));
        }
        shader
    }
}

fn load_program(gl: &glow::Context, src: &str) -> glow::Program {
    unsafe {
        let program = gl.create_program().expect("Cannot create program");
        gl.attach_shader(program, load_shader(gl, glow::VERTEX_SHADER, src));
        gl.attach_shader(program, load_shader(gl, glow::FRAGMENT_SHADER, src));
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }
        program
    }
}

fn slice_to_u8_slice<'a, T>(data: &[T]) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data)) }
}

fn main() {
    let rom_path = "roms/smb3.nes";
    let mut nes = nes001::NES001::from_rom(&std::fs::read(rom_path).unwrap());

    let mut player1_controller_state: ControllerState = ControllerState::new();
    let mut player2_controller_state: ControllerState = ControllerState::new();

    //*** AUDIO STUFF */
    let mut buffer_pos = 0;
    let mut buffer = Some(0);
    let mut w = platform::waveout::WaveoutDevice::new(8, 15720, 262);

    let mut waveout_callback = move |sample: i16| {
        if buffer.is_none() {
            if let Some(b) = w.get_current_buffer() {
                buffer = Some(b);
                buffer_pos = 0;
            }
        }

        if let Some(current_buffer) = buffer {
            w.buffers[current_buffer][buffer_pos] = sample;
            buffer_pos += 1;

            if buffer_pos == 262 {
                w.queue_buffer();
                buffer = None;
            }
        }
    };

    let mut wnd = platform::window::Window::new();
    let gl = wnd.create_gl_surface();

    let program = load_program(&gl, &std::fs::read_to_string("shaders/crt.glsl").unwrap());
    unsafe {
        gl.use_program(Some(program));

        gl.enable(glow::TEXTURE_2D);
        gl.disable(glow::CULL_FACE);

        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGB as i32,
            256,
            240,
            0,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            None,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32,
            256,
            240,
            0,
            glow::BGRA,
            glow::UNSIGNED_BYTE,
            None,
        );

        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        let vertices: [f32; 16] = [
            -1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 0.0, 1.0,
        ];

        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            slice_to_u8_slice(&vertices),
            glow::STATIC_DRAW,
        );

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let ibo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            slice_to_u8_slice(&indices),
            glow::STATIC_DRAW,
        );

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 4 * 4, 0);

        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 4 * 4, 2 * 4);
    }

    let dt_target = std::time::Duration::from_micros(16666);
    let mut last_time = std::time::Instant::now();
    let mut accum = std::time::Duration::ZERO;
    let mut sec_accum = std::time::Duration::ZERO;
    let one_second_duration = std::time::Duration::from_secs(1);
    let mut nes_frames = 0;
    let mut running = true;

    let mut gamepad = gamepad::Gamepad::new();

    while running {
        wnd.pump_events();
        gamepad.update_controller_state(&mut [&mut player1_controller_state, &mut player2_controller_state]);

        while let Some(event) = wnd.get_event() {
            use platform::window::WindowEvents::*;
            match event {
                Resize(width, height, size) => unsafe {
                    gl.viewport(width / 2 - size / 2, height / 2 - size / 2, size, size);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    wnd.swap_buffers();
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    wnd.swap_buffers();
                },
                Key(b'Q', down) => player1_controller_state.set_select(down),
                Key(b'W', down) => player1_controller_state.set_start(down),
                Key(b'A', down) => player1_controller_state.set_b(down),
                Key(b'S', down) => player1_controller_state.set_a(down),
                Key(37, down) => player1_controller_state.set_left(down),
                Key(38, down) => player1_controller_state.set_up(down),
                Key(39, down) => player1_controller_state.set_right(down),
                Key(40, down) => player1_controller_state.set_down(down),
                Key(116, true) => {
                    let save_path = format!("{}.sav", rom_path);
                    let mut buf_writer = BufWriter::new(std::fs::File::create(&save_path).unwrap());
                    nes.save(&mut buf_writer).unwrap();
                }
                Key(118, true) => {
                    let save_path = format!("{}.sav", rom_path);
                    let mut buf_reader = std::io::BufReader::new(std::fs::File::open(&save_path).unwrap());
                    nes.load(&mut buf_reader).unwrap();
                }
                Close => {
                    running = false;
                }
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

            wnd.set_title(format!("NES Emulator - FPS: {}", nes_fps).as_str());
        }

        while accum >= dt_target {
            nes.set_buttons_down(0, &player1_controller_state);
            nes.set_buttons_down(1, &player2_controller_state);
            nes.tick_frame(&mut waveout_callback);

            accum -= dt_target;

            nes_frames += 1;
        }

        unsafe {
            gl.clear(glow::COLOR_BUFFER_BIT);

            let u8_pixels = slice_to_u8_slice(&nes.framebuffer);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                256,
                240,
                glow::BGRA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(u8_pixels),
            );
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
        }

        wnd.swap_buffers();

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
