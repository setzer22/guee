use guee_derives::Builder;

use crate::{painter::Painter, prelude::*};

/// A container that forces to render its child widget with a specific maximum size.
#[derive(Builder)]
#[builder(widget)]
#[allow(clippy::type_complexity)]
pub struct CustomDrawContainer {
    pub contents: DynWidget,
    #[builder(skip)]
    pub pre_draw: Option<Box<dyn FnOnce(&mut Painter, &Layout)>>,
    #[builder(skip)]
    pub post_draw: Option<Box<dyn FnOnce(&mut Painter, &Layout)>>,
}

impl CustomDrawContainer {
    pub fn pre_draw<F: FnOnce(&mut Painter, &Layout) + 'static>(mut self, f: F) -> Self {
        self.pre_draw = Some(Box::new(f));
        self
    }

    pub fn post_draw<F: FnOnce(&mut Painter, &Layout) + 'static>(mut self, f: F) -> Self {
        self.post_draw = Some(Box::new(f));
        self
    }
}

impl Widget for CustomDrawContainer {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        self.contents
            .widget
            .layout(ctx, parent_id, available, force_shrink)
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        if let Some(pre_draw) = self.pre_draw.take() {
            (pre_draw)(&mut ctx.painter(), layout)
        }
        self.contents.widget.draw(ctx, layout);
        if let Some(post_draw) = self.post_draw.take() {
            (post_draw)(&mut ctx.painter(), layout)
        }
    }

    fn layout_hints(&self) -> LayoutHints {
        self.contents.widget.layout_hints()
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        self.contents
            .widget
            .on_event(ctx, layout, cursor_position, events)
    }
}
