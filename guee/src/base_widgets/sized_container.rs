use guee_derives::Builder;

use crate::prelude::*;

/// A container that forces to render its child widget with a specific maximum size.
#[derive(Builder)]
#[builder(widget)]
pub struct SizedContainer {
    contents: DynWidget,
    size: Vec2,
}

impl Widget for SizedContainer {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        _available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        self.contents
            .widget
            .layout(ctx, parent_id, self.size, force_shrink)
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        self.contents.widget.draw(ctx, layout)
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
        status: &mut EventStatus,
    ) {
        self.contents
            .widget
            .on_event(ctx, layout, cursor_position, events, status)
    }
}
