use crate::errors::Result;
use winit::WindowBuilder;
use winit::dpi::LogicalSize;
use winit::Window;
use winit::EventsLoop;
use crate::errors::ResultExt;
use crate::context::Context;
use crate::input::UserInput;
use crate::internal::graphics::GraphicsState;


#[derive(Debug)]
pub struct Starstruck {
    title: String,
    window: Window,
    events_loop: EventsLoop,
    graphics_state: GraphicsState
}

impl Starstruck {
    pub fn new(title: &str) -> Result<Self> {

        let events_loop = EventsLoop::new();
        let window = WindowBuilder::new()
            .with_title(title)
            .with_dimensions(LogicalSize {
                width: 800.0,
                height: 600.0
            })
            .build(&events_loop).chain_err(|| "Something failed")?;

        let graphics_state = GraphicsState::new(title, &window)?;

        Ok(Starstruck {
            title: title.to_string(),
            window,
            events_loop,
            graphics_state
        })
    }

    pub fn start_game_loop(mut self, callback: fn(context: &mut Context) -> ()) -> Result<()> {
        let events_loop = &mut self.events_loop;
        let graphics_state = &mut self.graphics_state;

        let mut user_input = UserInput::new();

        loop {
            {
                user_input.reset_and_poll_events(events_loop);
                let encoder = graphics_state.next_encoder()?;
                let mut context = Context::new(user_input, encoder);

                callback(&mut context);
            };


            graphics_state.present_swapchain()?;

            if user_input.end_requested {
                break;
            }
        }

        Ok(())
    }
}