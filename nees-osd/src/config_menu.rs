#[repr(u8)]
#[derive(Clone, Copy)]
pub enum StepResponse {
    None,
    SetButtonA { which_player: u8 },
    SetButtonB { which_player: u8 },
    SetButtonSelect { which_player: u8 },
    SetButtonStart { which_player: u8 },
    SetButtonUp { which_player: u8 },
    SetButtonDown { which_player: u8 },
    SetButtonLeft { which_player: u8 },
    SetButtonRight { which_player: u8 },
    SaveState,
    LoadState,
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum OSDAction {
    Up = 0,
    Down = 1,
    Ok = 2,
}

const PALETTE_COLORS: [u32; 4] = [0xFF111111, 0xFF555555, 0xFFAAAAAA, 0xFF882222];
const BACKGROUND: u8 = 0;
const GRAY: u8 = 1;
const WHITE: u8 = 2;
const BLUE: u8 = 3;
const START_ROW: u8 = 3;
const END_ROW: u8 = START_ROW + 14;

enum OSDState {
    Main { current_selection: u8 },
    RemapPlayer { which_player: u8, current_key: u8 },
    VideoSettings { current_selection: u8 },
}

#[allow(clippy::upper_case_acronyms)]
pub struct OSD {
    current_menu: OSDState,
}

impl OSD {
    pub fn new() -> Self {
        Self {
            current_menu: OSDState::Main {
                current_selection: 0,
            },
        }
    }

    fn draw_char(&self, framebuffer: &mut [u32], col: u8, row: u8, c: char, fg: u8, bg: u8) {
        let font = include_bytes!("../menu_font.bin");
        for yy in 0..8 {
            let mut bitpattern = font[(c as u16 * 8 + yy as u16) as usize];
            for xx in 0..8 {
                let palette_index = if bitpattern & 0x80 != 0 { fg } else { bg } as usize;
                bitpattern <<= 1;
                framebuffer
                    [(row as usize * 8 + yy as usize) * 256 + (col as usize * 8) + xx as usize] =
                    PALETTE_COLORS[palette_index];
            }
        }
    }

    fn draw_string(&self, framebuffer: &mut [u32], mut col: u8, row: u8, s: &str, fg: u8, bg: u8) {
        for c in s.chars() {
            self.draw_char(framebuffer, col, row, c, fg, bg);
            col += 1;
        }
    }

    fn draw_string_centered(&self, framebuffer: &mut [u32], row: u8, s: &str, fg: u8, bg: u8) {
        let col = (32 - s.len() as u8) / 2;
        self.draw_string(framebuffer, col, row, s, fg, bg);
    }

    fn clear_screen(&self, framebuffer: &mut [u32], bg: u8) {
        let palette_index = bg as usize;
        // for i in (START_ROW as usize) * 8 * 256..(END_ROW as usize) * 8 * 256 {
        //     framebuffer[i] = PALETTE_COLORS[palette_index];
        // }
        for y in (START_ROW as usize)*8..(END_ROW as usize)*8 {
            for x in 8..248 {
                framebuffer[(y * 256) + x] = PALETTE_COLORS[palette_index];
            }
        }
    }

    fn draw_menu_item(&self, framebuffer: &mut [u32], row: u8, text: &str, selected: bool) {
        let bg = if selected { BLUE } else { BACKGROUND };
        self.draw_string(framebuffer, 1, row, text, WHITE, bg);
        if selected {
            self.draw_char(framebuffer, 1, row, '\x10', WHITE, bg);
        }
    }

    pub fn draw_step(&mut self, framebuffer: &mut [u32]) {
        self.clear_screen(framebuffer, BACKGROUND);
        self.draw_string_centered(framebuffer, START_ROW+1, "NEES Options", WHITE, BACKGROUND);

        match self.current_menu {
            OSDState::Main { current_selection } => {
                self.draw_string_centered(
                    framebuffer,
                    START_ROW+3,
                    "Use arrow keys to navigate",
                    GRAY,
                    BACKGROUND,
                );
                self.draw_string_centered(
                    framebuffer,
                    START_ROW+4,
                    "Any other button to select",
                    GRAY,
                    BACKGROUND,
                );
                self.draw_menu_item(framebuffer, START_ROW+6, "  Remap player 1", current_selection == 0);
                self.draw_menu_item(framebuffer, START_ROW+7, "  Remap player 2", current_selection == 1);
                self.draw_menu_item(framebuffer, START_ROW+9, "  Video settings", current_selection == 2);
                self.draw_menu_item(framebuffer, START_ROW+11, "  Save state", current_selection == 3);
                self.draw_menu_item(framebuffer, START_ROW+12, "  Load state", current_selection == 4);
            }
            OSDState::RemapPlayer {
                which_player: _,
                current_key,
            } => match current_key {
                0 => self.draw_string_centered(framebuffer, START_ROW+6, "Press B", WHITE, BACKGROUND),
                1 => self.draw_string_centered(framebuffer, START_ROW+6, "Press A", WHITE, BACKGROUND),
                2 => self.draw_string_centered(framebuffer, START_ROW+6, "Press Select", WHITE, BACKGROUND),
                3 => self.draw_string_centered(framebuffer, START_ROW+6, "Press Start", WHITE, BACKGROUND),
                4 => self.draw_string_centered(framebuffer, START_ROW+6, "Press Up", WHITE, BACKGROUND),
                5 => self.draw_string_centered(framebuffer, START_ROW+6, "Press Down", WHITE, BACKGROUND),
                6 => self.draw_string_centered(framebuffer, START_ROW+6, "Press Left", WHITE, BACKGROUND),
                7 => self.draw_string_centered(framebuffer, START_ROW+6, "Press Right", WHITE, BACKGROUND),
                _ => {}
            },
            OSDState::VideoSettings {
                current_selection,
            } => {
                self.draw_menu_item(framebuffer, START_ROW+6, "  Horizontal adjustment", current_selection == 0);
                self.draw_menu_item(framebuffer, START_ROW+7, "  Curve ratio", current_selection == 1);
                self.draw_menu_item(framebuffer, START_ROW+8, "  Scanlines", current_selection == 2);
                self.draw_menu_item(framebuffer, START_ROW+10, "  Back", current_selection == 3);
            }
        }
    }

    pub fn step(&mut self, action: OSDAction) -> StepResponse {
        match self.current_menu {
            OSDState::Main { current_selection } => match action {
                OSDAction::Up => {
                    self.current_menu = OSDState::Main {
                        current_selection: if current_selection == 0 {
                            4
                        } else {
                            current_selection - 1
                        },
                    };
                }
                OSDAction::Down => {
                    self.current_menu = OSDState::Main {
                        current_selection: if current_selection == 4 {
                            0
                        } else {
                            current_selection + 1
                        },
                    };
                }
                OSDAction::Ok => match current_selection {
                    0 => {
                        self.current_menu = OSDState::RemapPlayer {
                            which_player: 0,
                            current_key: 0,
                        };
                    }
                    1 => {
                        self.current_menu = OSDState::RemapPlayer {
                            which_player: 1,
                            current_key: 0,
                        };
                    }
                    2 => {
                        self.current_menu = OSDState::VideoSettings {
                            current_selection: 0,
                        };
                    }
                    3 => return StepResponse::SaveState,
                    4 => return StepResponse::LoadState,
                    _ => {}
                },
            },
            OSDState::RemapPlayer {
                which_player,
                current_key,
            } => {
                let response = match current_key {
                    0 => StepResponse::SetButtonB { which_player },
                    1 => StepResponse::SetButtonA { which_player },
                    2 => StepResponse::SetButtonSelect { which_player },
                    3 => StepResponse::SetButtonStart { which_player },
                    4 => StepResponse::SetButtonUp { which_player },
                    5 => StepResponse::SetButtonDown { which_player },
                    6 => StepResponse::SetButtonLeft { which_player },
                    7 => StepResponse::SetButtonRight { which_player },
                    _ => panic!("Invalid"),
                };

                if current_key == 7 {
                    self.current_menu = OSDState::Main {
                        current_selection: 0,
                    };
                } else {
                    self.current_menu = OSDState::RemapPlayer {
                        which_player,
                        current_key: current_key + 1,
                    };
                }

                return response;
            }
            OSDState::VideoSettings {
                current_selection,
            } => match action {
                OSDAction::Up => {
                    self.current_menu = OSDState::VideoSettings {
                        current_selection: if current_selection == 0 {
                            3
                        } else {
                            current_selection - 1
                        },
                    };
                }
                OSDAction::Down => {
                    self.current_menu = OSDState::VideoSettings {
                        current_selection: if current_selection == 3 {
                            0
                        } else {
                            current_selection + 1
                        },
                    };
                }
                OSDAction::Ok => match current_selection {
                    3 => {
                        self.current_menu = OSDState::Main {
                            current_selection: 2,
                        };
                    }
                    _ => {}
                },
            },
        }

        StepResponse::None
    }
}

impl Default for OSD {
    fn default() -> Self {
        Self::new()
    }
}
