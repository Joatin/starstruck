use std::collections::HashSet;
use winit::DeviceEvent;
use winit::ElementState;
use winit::Event;
use winit::EventsLoop;
use winit::KeyboardInput;
use winit::VirtualKeyCode;
use winit::WindowEvent;

#[derive(Debug, Clone, Default)]
pub struct UserInput {
    pub end_requested: bool,
    pub resized: bool,
    pub keys_held: HashSet<VirtualKeyCode>,
    pub keys_clicked: HashSet<VirtualKeyCode>,
    pub mouse_position: (f64, f64),
}

impl UserInput {
    pub fn new() -> Self {
        UserInput::default()
    }

    #[allow(clippy::single_match)]
    pub fn reset_and_poll_events(&mut self, events_loop: &mut EventsLoop) {
        events_loop.poll_events(|event| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => self.end_requested = true,
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => self.resized = true,
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                self.mouse_position = (position.x, position.y);
            }
            // Track all keys, all the time. Note that because of key rollover details
            // it's possible to get key released events for keys we don't think are
            // pressed. This is a hardware limit, not something you can evade.
            Event::DeviceEvent {
                event:
                    DeviceEvent::Key(KeyboardInput {
                        virtual_keycode: Some(code),
                        state,
                        ..
                    }),
                ..
            } => match state {
                ElementState::Pressed => {
                    self.keys_held.insert(code);
                }
                ElementState::Released => {
                    self.keys_held.remove(&code);
                    self.keys_clicked.insert(code);
                }
            },

            // We want to respond to some of the keys specially when they're also
            // window events too (meaning that the window was focused when the event
            // happened).
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(code),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                {
                    match state {
                        ElementState::Pressed => {
                            self.keys_held.insert(code);
                        }
                        ElementState::Released => {
                            self.keys_held.remove(&code);
                            self.keys_clicked.insert(code);
                        }
                    };
                };
            }

            _ => (),
        });
    }

    pub fn flush(&mut self) {
        self.keys_clicked.clear();
        self.resized = false;
        self.end_requested = false;
    }
}
