use crate::context::Context;
use crate::menu::View;
use crate::setup_context::SetupContext;
use failure::Error;
use futures::Future;
use std::sync::Arc;
use crate::allocator::GpuAllocator;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use crate::internal::graphics::TextManager;
use futures::future::IntoFuture;
use gfx_hal::window::Extent2D;
use glyph_brush::Layout;
use glyph_brush::BuiltInLineBreaker;

pub struct MenuManager<'a, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    view: Option<Arc<View<A, B, D, I>>>,
    loading_view: Arc<View<A, B, D, I>>,
    show_loading_view: bool,
    text_manager: TextManager<'a, A, B, D, I>
}

impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> MenuManager<'static, A, B, D, I> {
    pub(crate) fn new(
        setup_context: Arc<SetupContext<A, B, D, I>>,
        loading_view: Arc<View<A, B, D, I>>,
    ) -> impl Future<Item = Self, Error = Error> {

        let text_manager_future = TextManager::new(Arc::clone(&setup_context)).into_future();

        text_manager_future
            .map(|text_manager| Self {
                view: None,
                loading_view,
                show_loading_view: true,
                text_manager
            })
    }

    pub fn resize(&mut self, _area: Extent2D) {
        self.text_manager.resize()
    }

    pub fn draw_text(&mut self, text: &str, size: f32, screen_position: (f32, f32), layout: Layout<BuiltInLineBreaker>) {
        self.text_manager.draw_text(text, size, screen_position, layout);
    }

    pub fn hide_loading_view(&mut self) {
        self.show_loading_view = false;
    }

    pub fn _display(&mut self, view: Option<Arc<View<A, B, D, I>>>) {
        self.view = view;
    }

    pub(crate) fn draw(&mut self, context: &mut Context<A, B, D, I>) -> Result<(), Error> {
        self.text_manager.draw(context)?;

        if self.show_loading_view {
            self.draw_view(context, &self.loading_view)
        } else if let Some(view) = self.view.as_ref() {
            self.draw_view(context, view)
        } else {
            Ok(())
        }
    }

    pub(crate) fn should_draw_content(&self) -> bool {
        if self.show_loading_view {
            self.loading_view.covers_screen()
        } else if let Some(view) = self.view.as_ref() {
            !view.covers_screen()
        } else {
            true
        }
    }

    fn draw_view(&self, context: &mut Context<A, B, D, I>, view: &Arc<View<A, B, D, I>>) -> Result<(), Error> {
        view.draw(context)?;
        Ok(())
    }
}
