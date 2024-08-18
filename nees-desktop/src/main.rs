//#![windows_subsystem = "windows"]

use nees::nes001;
use nes001::ControllerState;
use platform::window::Menu;

mod gamepad;
mod platform;

fn main() {
    let rom_path = "roms/ns.nes";
    let mut nes = nes001::NES001::from_rom(&std::fs::read(rom_path).unwrap());

    let mut player1_controller_state: ControllerState = ControllerState::new();
    let mut player2_controller_state: ControllerState = ControllerState::new();

    //*** AUDIO STUFF */
    let mut w = platform::waveout::WaveoutDevice::new(8, 15720, 262);
    let mut waveout_callback = move |sample: i16| {
        w.push_sample(sample);
    };

    let mut wnd = platform::window::Window::new("Nees", 512, 512);
    wnd.add_menus(&[
        Menu::Popout {
            title: "File",
            children: &[
                Menu::Item {
                    id: 1,
                    title: "Load ROM...",
                },
                Menu::Separator,
                Menu::Item {
                    id: 2,
                    title: "Exit",
                },
            ],
        },
        Menu::Popout {
            title: "Options",
            children: &[
                Menu::Item {
                    id: 3,
                    title: "Save State\tF5",
                },
                Menu::Item {
                    id: 4,
                    title: "Load State\tF7",
                },
            ],
        },
    ]);

    let gl = wnd.create_gl_surface();
    nees_glrenderer::init(&gl);

    let dt_target = std::time::Duration::from_micros(16666);
    let mut last_time = std::time::Instant::now();
    let mut accum = std::time::Duration::ZERO;
    let mut sec_accum = std::time::Duration::ZERO;
    let one_second_duration = std::time::Duration::from_secs(1);
    let mut nes_frames = 0;
    let mut running = true;

    let mut gamepad = gamepad::Gamepad::new();
    let mut framebuffer: Vec<u32> = vec![0; 256 * 240];

    while running {
        wnd.pump_events();
        gamepad.update_controller_state(&mut [
            &mut player1_controller_state,
            &mut player2_controller_state,
        ]);

        while let Some(event) = wnd.get_event() {
            use platform::keys::*;
            use platform::window::WindowEvents::*;

            match event {
                Resize(width, height) => {
                    nees_glrenderer::resize(&gl, width, height);
                    wnd.swap_buffers();
                }
                Key(b'Q', down) => player1_controller_state.set_select(down),
                Key(b'W', down) => player1_controller_state.set_start(down),
                Key(b'A', down) => player1_controller_state.set_b(down),
                Key(b'S', down) => player1_controller_state.set_a(down),
                Key(ARROW_LEFT, down) => player1_controller_state.set_left(down),
                Key(ARROW_UP, down) => player1_controller_state.set_up(down),
                Key(ARROW_RIGHT, down) => player1_controller_state.set_right(down),
                Key(ARROW_DOWN, down) => player1_controller_state.set_down(down),
                Key(F5, true) => {
                    nees_std::save_state(rom_path, &nes);
                }
                Key(F7, true) => {
                    nees_std::load_state(rom_path, &mut nes);
                }
                Command { which: 3 } => {
                    nees_std::save_state(rom_path, &nes);
                }
                Command { which: 4 } => {
                    nees_std::load_state(rom_path, &mut nes);
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

            wnd.set_title(format!("Nees - FPS: {}", nes_fps).as_str());
        }

        while accum >= dt_target {
            nes.set_buttons_down(0, &player1_controller_state);
            nes.set_buttons_down(1, &player2_controller_state);
            nes.tick_frame(&mut waveout_callback, &mut framebuffer);

            accum -= dt_target;

            nes_frames += 1;
        }

        nees_glrenderer::render(&gl, &framebuffer);

        wnd.swap_buffers();

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
