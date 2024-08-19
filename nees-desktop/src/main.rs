//#![windows_subsystem = "windows"]

use nees::nes001;
use nes001::ControllerState;
use platform::{
    keys,
    window::{Menu, WindowEvents},
};

mod gamepad;
mod platform;

fn main() {
    let rom_path = "roms/smb3.nes";
    let mut nes = nes001::NES001::from_rom(&std::fs::read(rom_path).unwrap());

    let mut controller_states: [ControllerState; 2] =
        [ControllerState::new(), ControllerState::new()];

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

    let mut osd = nees_osd::config_menu::OSD::new();
    let mut osd_open = false;

    let mut player_select_key = [b'K', b'Q'];
    let mut player_start_key = [b'L', b'W'];
    let mut player_b_key = [0xBC, b'A'];
    let mut player_a_key = [0xBE, b'S'];
    let mut player_up_key = [keys::ARROW_UP, b'T'];
    let mut player_down_key = [keys::ARROW_DOWN, b'G'];
    let mut player_left_key = [keys::ARROW_LEFT, b'F'];
    let mut player_right_key = [keys::ARROW_RIGHT, b'H'];

    while running {
        wnd.pump_events();
        gamepad.update_controller_state(&mut controller_states);

        while let Some(event) = wnd.get_event() {
            use platform::keys::*;
            use platform::window::WindowEvents::*;

            match event {
                Resize(width, height) => {
                    nees_glrenderer::resize(&gl, width, height);
                    wnd.swap_buffers();
                }
                Key(key, down) if key == player_select_key[0] => {
                    controller_states[0].set_select(down)
                }
                Key(key, down) if key == player_start_key[0] => {
                    controller_states[0].set_start(down)
                }
                Key(key, down) if key == player_b_key[0] => controller_states[0].set_b(down),
                Key(key, down) if key == player_a_key[0] => controller_states[0].set_a(down),
                Key(key, down) if key == player_up_key[0] => controller_states[0].set_up(down),
                Key(key, down) if key == player_down_key[0] => controller_states[0].set_down(down),
                Key(key, down) if key == player_left_key[0] => controller_states[0].set_left(down),
                Key(key, down) if key == player_right_key[0] => {
                    controller_states[0].set_right(down)
                }

                Key(key, down) if key == player_select_key[1] => {
                    controller_states[1].set_select(down)
                }
                Key(key, down) if key == player_start_key[1] => {
                    controller_states[1].set_start(down)
                }
                Key(key, down) if key == player_b_key[1] => controller_states[1].set_b(down),
                Key(key, down) if key == player_a_key[1] => controller_states[1].set_a(down),
                Key(key, down) if key == player_up_key[1] => controller_states[1].set_up(down),
                Key(key, down) if key == player_down_key[1] => controller_states[1].set_down(down),
                Key(key, down) if key == player_left_key[1] => controller_states[1].set_left(down),
                Key(key, down) if key == player_right_key[1] => {
                    controller_states[1].set_right(down)
                }
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

            if let WindowEvents::Key(key, true) = event {
                if key == keys::ESCAPE {
                    osd_open = !osd_open;
                    osd.draw_step(&mut framebuffer);
                } else if osd_open {
                    let response = if key == ARROW_UP {
                        osd.step(nees_osd::config_menu::OSDAction::Up)
                    } else if key == ARROW_DOWN {
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
                        nees_osd::config_menu::StepResponse::SaveState => todo!(),
                        nees_osd::config_menu::StepResponse::LoadState => todo!(),
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

                wnd.set_title(format!("Nees - FPS: {}", nes_fps).as_str());
            }

            while accum >= dt_target {
                nes.set_buttons_down(0, &controller_states[0]);
                nes.set_buttons_down(1, &controller_states[1]);
                nes.tick_frame(&mut waveout_callback, &mut framebuffer);

                accum -= dt_target;

                nes_frames += 1;
            }
        }

        nees_glrenderer::render(&gl, &framebuffer);

        wnd.swap_buffers();

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
