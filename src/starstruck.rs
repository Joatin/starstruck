use crate::context::Context;
use crate::errors::CreateEncoderErrorKind;
use crate::input::UserInput;
use crate::internal::graphics::GraphicsState;
use crate::setup_context::SetupContext;
use colored::*;
use failure::Error;
use futures::lazy;
use futures::Future;
use futures::IntoFuture;
use gfx_hal::device::Device;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Instant;
use winit::EventsLoop;
use winit::Window;
use winit::WindowBuilder;

const BANNER: &str = "

 $$$$$$\\    $$\\                                     $$\\                                   $$\\
$$  __$$\\   $$ |                                    $$ |                                  $$ |
$$ /  \\__|$$$$$$\\    $$$$$$\\   $$$$$$\\   $$$$$$$\\ $$$$$$\\    $$$$$$\\  $$\\   $$\\  $$$$$$$\\ $$ |  $$\\
\\$$$$$$\\  \\_$$  _|   \\____$$\\ $$  __$$\\ $$  _____|\\_$$  _|  $$  __$$\\ $$ |  $$ |$$  _____|$$ | $$  |
 \\____$$\\   $$ |     $$$$$$$ |$$ |  \\__|\\$$$$$$\\    $$ |    $$ |  \\__|$$ |  $$ |$$ /      $$$$$$  /
$$\\   $$ |  $$ |$$\\ $$  __$$ |$$ |       \\____$$\\   $$ |$$\\ $$ |      $$ |  $$ |$$ |      $$  _$$<
\\$$$$$$  |  \\$$$$  |\\$$$$$$$ |$$ |      $$$$$$$  |  \\$$$$  |$$ |      \\$$$$$$  |\\$$$$$$$\\ $$ | \\$$\\
 \\______/    \\____/  \\_______|\\__|      \\_______/    \\____/ \\__|       \\______/  \\_______|\\__|  \\__|

 ";

pub struct Starstruck<State, RenderCallback> {
    title: String,
    window: Window,
    events_loop: EventsLoop,
    graphics_state: Arc<GraphicsState>,
    setup_context: Arc<SetupContext>,
    setup_callback: Option<Box<Future<Item = State, Error = Error> + Send>>,
    render_callback: RenderCallback,
}

impl<'a, State: 'static + Send + Sync, RenderCallback> Starstruck<State, RenderCallback>
where
    RenderCallback: FnMut((&mut State, &mut Context)) -> Result<(), Error>,
{
    pub fn init<C, F, FI>(
        title: &str,
        setup_callback: C,
        render_callback: RenderCallback,
    ) -> Result<Self, Error>
    where
        C: Send + 'static + FnOnce(Arc<SetupContext>) -> F,
        F: IntoFuture<Future = FI, Item = State, Error = Error> + 'static,
        FI: Future<Item = State, Error = Error> + Send + 'static,
    {
        Self::print_banner();
        info!("Initializing starstruck engine");

        info!("Creating new window");
        let events_loop = EventsLoop::new();
        let window = WindowBuilder::new().with_title(title).build(&events_loop)?;

        let graphics_state = Arc::new(GraphicsState::new(title, &window)?);
        let context = Arc::new(SetupContext::new(Arc::clone(&graphics_state)));

        let s_callback = {
            let cloned_context = Arc::clone(&context);
            let future = Box::new(lazy(move || setup_callback(cloned_context)))
                as Box<Future<Item = State, Error = Error> + Send>;
            Some(future)
        };

        Ok(Self {
            title: title.to_string(),
            window,
            events_loop,
            graphics_state,
            setup_context: context,
            setup_callback: s_callback,
            render_callback,
        })
    }

    pub fn run(mut self) -> Result<(), Error> {
        let events_loop = &mut self.events_loop;
        let graphics_state = &mut self.graphics_state;
        let setup = self.setup_callback.take().unwrap();
        let s_context = &self.setup_context;
        let render_callback = &mut self.render_callback;

        let mut user_input = UserInput::new();

        let s_data = Arc::new(RwLock::new(None));

        let cloned_data = Arc::clone(&s_data);
        thread::spawn(move || {
            let now = Instant::now();
            let r: State = tokio::runtime::current_thread::block_on_all(setup).unwrap();
            let mut d = cloned_data.write().unwrap();
            d.replace(r);
            info!(
                "{}",
                format!("Setup took {:?} to complete", now.elapsed()).magenta()
            )
        });

        let mut recreate_swapchain = false;

        loop {
            let render_area = graphics_state.render_area();

            if recreate_swapchain {
                graphics_state.device().wait_idle()?;
                s_context.drop_swapchain_dependant_data();
                graphics_state.recreate_swapchain(&self.window)?;
                s_context.recreate_swapchain_dependant_data()?;
                recreate_swapchain = false;
            }
            user_input.reset_and_poll_events(events_loop);

            let user_input_clone = user_input.clone();
            {
                if let Err(error) = graphics_state.next_encoder(|encoder| {
                    let mut context =
                        Context::new(user_input_clone, &s_context, encoder, render_area);

                    {
                        let mut guard = s_data.write().unwrap();
                        if let Some(d) = guard.as_mut() {
                            render_callback((d, &mut context))?;
                        }
                    }
                    Ok(())
                }) {
                    match error.kind() {
                        CreateEncoderErrorKind::RecreateSwapchain => {
                            recreate_swapchain = true;
                            user_input.flush();
                            continue;
                        }
                        CreateEncoderErrorKind::Timeout => continue,
                        _ => bail!(error),
                    }
                };
            };

            graphics_state.present_swapchain()?;

            if user_input.resized {
                recreate_swapchain = true;
            }

            if user_input.end_requested {
                break;
            }

            user_input.flush();
        }

        Ok(())
    }

    fn print_banner() {
        println!("{}", BANNER.green());
    }
}

impl<State, RenderCallback> Debug for Starstruck<State, RenderCallback> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.title)?;
        write!(f, "Window: {:?}", self.window)?;
        write!(f, "EventsLoop: {:?}", self.events_loop)
    }
}
