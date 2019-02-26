use winit::EventsLoop;
use winit::Event;
use winit::WindowEvent;

#[derive(Debug, Clone, Copy, Default)]
pub struct UserInput {
    pub end_requested: bool,
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

            _ => ()
        });
    }

}