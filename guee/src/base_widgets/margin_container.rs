use epaint::{Pos2, Vec2};
use guee_derives::Builder;

use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints},
    widget::{DynWidget, Widget},
};

#[derive(Builder)]
pub struct MarginContainer {
    #[builder(default)]
    margin: Vec2,
    contents: DynWidget,
}

impl Widget for MarginContainer {
    fn layout(&mut self, ctx: &Context, available: Vec2) -> Layout {
        let mut content_layout = self.contents.widget.layout(ctx, available - self.margin);
        content_layout.translate(self.margin * 0.5);
        Layout::with_children(
            content_layout.bounds.size() + self.margin,
            vec![content_layout],
        )
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        self.contents.widget.draw(ctx, &layout.children[0])
    }

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        self.contents.widget.min_size(ctx, available - self.margin) + self.margin
    }

    fn layout_hints(&self) -> LayoutHints {
        self.contents.widget.layout_hints()
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        event: &Event,
    ) -> EventStatus {
        self.contents
            .widget
            .on_event(ctx, &layout.children[0], cursor_position, event)
    }
}
