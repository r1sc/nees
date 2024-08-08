use gilrs::{Button, Event, EventType, Gilrs};
use nees::nes001::ControllerState;

pub struct Gamepad {
    gilrs: Gilrs,
    player1_id: Option<gilrs::GamepadId>,
    player2_id: Option<gilrs::GamepadId>,
}

impl Gamepad {
    pub fn new() -> Self {
        Self {
            gilrs: Gilrs::new().unwrap(),
            player1_id: None,
            player2_id: None,
        }
    }

    pub fn update_controller_state(&mut self, controller_states: &mut [&mut ControllerState; 2]) {
        while let Some(Event { id, event, time }) = self.gilrs.next_event() {
            if event == EventType::Connected {
                println!("Gamepad {} connected at {:?}", id, time);

                if self.player1_id.is_none() {
                    self.player1_id = Some(id);
                } else if self.player2_id.is_none() && self.player1_id != Some(id) {
                    self.player2_id = Some(id);
                }
            } else if event == EventType::Disconnected {
                println!("Gamepad {} disconnected at {:?}", id, time);

                if self.player1_id == Some(id) {
                    self.player1_id = None;
                } else if self.player2_id == Some(id) {
                    self.player2_id = None;
                }
            } else if self.player1_id == Some(id) || self.player2_id == Some(id) {
                let player_id = if id == self.player1_id.unwrap() { 0 } else { 1 };

                let controller_state = &mut controller_states[player_id];

                match event {
                    EventType::ButtonPressed(button, _) => match button {
                        Button::South => controller_state.set_a(true),
                        Button::West => controller_state.set_b(true),
                        Button::Select => controller_state.set_select(true),
                        Button::Start => controller_state.set_start(true),
                        _ => {}
                    },
                    EventType::ButtonReleased(button, _) => match button {
                        Button::South => controller_state.set_a(false),
                        Button::West => controller_state.set_b(false),
                        Button::Select => controller_state.set_select(false),
                        Button::Start => controller_state.set_start(false),
                        _ => {}
                    },
                    EventType::AxisChanged(axis, value, _) => match axis {
                        gilrs::Axis::LeftStickX => {
                            controller_state.set_left(value < -0.5);
                            controller_state.set_right(value > 0.5);
                        }
                        gilrs::Axis::LeftStickY => {
                            controller_state.set_down(value < -0.5);
                            controller_state.set_up(value > 0.5);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
}
