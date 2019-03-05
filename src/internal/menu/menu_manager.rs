use crate::context::Context;
use crate::graphics::Bundle;
use crate::graphics::Pipeline;
use crate::menu::View;
use crate::primitive::Vertex2D;
use crate::setup_context::CreateDefaultPipeline;
use crate::setup_context::SetupContext;
use failure::Error;
use futures::Future;
use std::sync::Arc;

// OUR VERTICES
const VERTICES: [Vertex2D; 4] = [
    Vertex2D { x: -1.0, y: 1.0 },
    Vertex2D { x: 1.0, y: -1.0 },
    Vertex2D { x: 1.0, y: 1.0 },
    Vertex2D { x: -1.0, y: -1.0 },
];

// INDEXES
const INDEXES: [u16; 6] = [0, 1, 2, 3, 0, 1];

pub struct MenuManager {
    view: Option<Arc<View>>,
    loading_view: Arc<View>,
    show_loading_view: bool,
    bundle: Bundle<u16, Vertex2D>,
    pipeline: Arc<Pipeline<Vertex2D>>,
}

impl MenuManager {
    pub(crate) fn new(
        setup_context: Arc<SetupContext>,
        loading_view: Arc<View>,
    ) -> impl Future<Item = Self, Error = Error> {
        let pipeline_future = setup_context.create_default_pipeline();
        let bundle_future = setup_context.create_bundle(&INDEXES, &VERTICES);

        pipeline_future
            .join(bundle_future)
            .map(|(pipeline, bundle)| Self {
                view: None,
                loading_view,
                show_loading_view: true,
                bundle,
                pipeline,
            })
    }

    pub fn hide_loading_view(&mut self) {
        self.show_loading_view = false;
    }

    pub fn display(&mut self, view: Option<Arc<View>>) {
        self.view = view;
    }

    pub(crate) fn draw(&self, context: &mut Context) -> Result<bool, Error> {
        if self.show_loading_view {
            self.draw_view(context, &self.loading_view)
        } else if let Some(view) = self.view.as_ref() {
            self.draw_view(context, view)
        } else {
            Ok(true)
        }
    }

    fn draw_view(&self, context: &mut Context, view: &Arc<View>) -> Result<bool, Error> {
        context.draw(&self.pipeline, &self.bundle);
        view.draw(context);
        Ok(!view.covers_screen())
    }
}