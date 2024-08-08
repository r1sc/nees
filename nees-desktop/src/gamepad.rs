use gilrs::{Button, Event, EventType, Gilrs};
use nees::nes001::ControllerState;

pub struct Gamepad {
    gilrs: Gilrs,
}

impl Gamepad {
    pub fn new() -> Self {
        Self {
            gilrs: Gilrs::new().unwrap(),
        }
    }

    pub fn update_controller_state(&mut self, controller_state: &mut ControllerState) {
        while let Some(Event { id, event, time }) = self.gilrs.next_event() {
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
                EventType::AxisChanged(axis, value, _) => {
                    match axis {
                        gilrs::Axis::LeftStickX => {
                            controller_state.set_left(value < -0.5);
                            controller_state.set_right(value > 0.5);
                        }
                        gilrs::Axis::LeftStickY => {
                            controller_state.set_down(value < -0.5);
                            controller_state.set_up(value > 0.5);
                        }
                        _ => {}
                    }
                }
                EventType::Connected => {
                    println!("Gamepad {} connected at {:?}", id, time);
                }
                EventType::Disconnected => {
                    println!("Gamepad {} disconnected at {:?}", id, time);
                }
                _ => {}
            }
        }
    }
}
