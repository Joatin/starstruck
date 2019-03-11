use crate::context::Context;
use crate::errors::CreateEncoderErrorKind;
use crate::input::UserInput;
use crate::internal::graphics::GraphicsState;
use crate::internal::menu::InitView;
use crate::internal::menu::MenuManager;
use crate::setup_context::SetupContext;
use colored::*;
use failure::Error;
use futures::lazy;
use futures::Future;
use futures::IntoFuture;
use gfx_hal::device::Device;
use gfx_hal::Backend;
use gfx_hal::Instance;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use winit::EventsLoop;
use winit::Window;
use winit::WindowBuilder;
use std::sync::mpsc::channel;
use crate::allocator::GpuAllocator;

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

pub trait State: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> State for T {}


/// The main struct used when writing a starstruck application.
#[allow(clippy::type_complexity)]
pub struct Starstruck<S: State, A: GpuAllocator<B, D>, B: Backend = backend::Backend, D: Device<B> = backend::Device, I: Instance<Backend = B> = backend::Instance> {
    title: String,
    window: Window,
    events_loop: EventsLoop,
    graphics_state: Arc<GraphicsState<A, B, D, I>>,
    setup_context: Arc<SetupContext<A, B, D, I>>,
    setup_callback: Option<Box<Future<Item = S, Error = Error> + Send>>,
    render_callback: Box<FnMut(
        (
            &mut S,
            &mut Context<A, B, D, I>,
        ),
    ) -> Result<(), Error>>
}

impl<'a, S: State, A: GpuAllocator>
    Starstruck<S, A, backend::Backend, backend::Device, backend::Instance>
{
    /// Initializes a new Starstruck instance
    ///
    /// # Arguments
    ///
    /// * `title` - The name used for this app. This is also displayed in the window on operating systems that supports it
    /// * `setup_callback` - Callback used to setup all needed dependencies
    /// * `render_callback` - Called on each render loop. Used to draw the app
    ///
    /// # Errors
    ///
    /// Result might contain an error if something went wrong during setup
    #[allow(clippy::type_complexity)]
    pub(crate) fn init<R: Future<Item = S, Error = Error> + Send + 'static, I: IntoFuture<Future = R, Item = S, Error = Error> + 'static>(
        title: &str,
        mut setup_callback: Box<FnMut(Arc<SetupContext<A>>) -> I + Send>,
        render_callback: Box<FnMut(
            (
                &mut S,
                &mut Context<A, backend::Backend, backend::Device, backend::Instance>,
            ),
        ) -> Result<(), Error>>,
        allocator: A
    ) -> Result<Self, Error>
    {
        Self::print_banner();
        info!("Initializing starstruck engine");

        info!("Creating new window");
        let events_loop = EventsLoop::new();
        let window = WindowBuilder::new().with_title(title).build(&events_loop)?;

        let graphics_state = Arc::new(GraphicsState::new(title, &window, allocator)?);
        let context = Arc::new(SetupContext::new(Arc::clone(&graphics_state)));

        let s_callback = {
            let cloned_context = Arc::clone(&context);
            let future = Box::new(lazy(move || setup_callback(cloned_context)))
                as Box<Future<Item = S, Error = Error> + Send>;
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

    /// Starts starstruck and enters the render loop
    ///
    /// Observer that this will consume the thread
    ///
    /// # Errors
    ///
    /// Result might contain an error if something went wrong during the render loop
    ///
    /// # Examples
    ///
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use starstruck::StarstruckBuilder;
    ///
    /// let starstruck = StarstruckBuilder::new().init()?;
    ///
    /// // starstruck.run()?; <- This will consume the thread
    /// # Ok(())
    /// # }
    /// ```
    pub fn run(mut self) -> Result<(), Error> {
        let initial_view = Arc::new(InitView::new()?);
        let mut menu_manager =
            MenuManager::new(Arc::clone(&self.setup_context), initial_view).wait()?;
        let events_loop = &mut self.events_loop;
        let graphics_state = &mut self.graphics_state;
        let setup = self.setup_callback.take().unwrap();
        let s_context = &self.setup_context;
        let render_callback = &mut self.render_callback;

        let mut user_input = UserInput::new();

        let (sender, receiver) = channel();

        thread::spawn(move || {
            let now = Instant::now();
            let r: S = setup.wait().unwrap();
            info!(
                "{}",
                format!("Setup took {:?} to complete", now.elapsed()).magenta()
            );
            sender.send(r).unwrap();
        });

        let mut recreate_swapchain = false;
        let mut end_requested = false;
        let mut state = None;

        info!("Entering render loop");
        loop {
            let render_area = graphics_state.render_area();
            if state.is_none() {
                state = receiver.try_recv().ok();
                if state.is_some() {
                    menu_manager.hide_loading_view();
                }
            }

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

                    if menu_manager.draw(&mut context)? {
                        if let Some(d) = state.as_mut() {
                            render_callback((d, &mut context))?;
                        }
                    }

                    if context.should_stop_starstruck() {
                        end_requested = true;
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

            if user_input.end_requested || end_requested {
                info!("Stopping starstruck");
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

impl<S: State, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug
    for Starstruck<S, A, B, D, I>
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.title)?;
        write!(f, "Window: {:?}", self.window)?;
        write!(f, "EventsLoop: {:?}", self.events_loop)
    }
}
