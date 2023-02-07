use epaint::{Pos2, Vec2};
use guee_derives::Builder;

use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints},
    widget::{DynWidget, Widget},
    widget_id::{IdGen, WidgetId},
};

#[derive(Builder)]
#[builder(widget)]
pub struct MarginContainer {
    id: IdGen,
    #[builder(default)]
    margin: Vec2,
    contents: DynWidget,
}

impl Widget for MarginContainer {
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        let widget_id = self.id.resolve(parent_id);

        let mut content_layout =
            self.contents
                .widget
                .layout(ctx, widget_id, available - self.margin);
        content_layout.translate(self.margin * 0.5);
        Layout::with_children(
            widget_id,
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
        events: &[Event],
    ) -> EventStatus {
        self.contents
            .widget
            .on_event(ctx, &layout.children[0], cursor_position, events)
    }
}
