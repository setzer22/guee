use epaint::{Color32, Pos2, RectShape, Rounding, Stroke, Vec2};
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
    #[builder(default = Color32::TRANSPARENT)]
    background_color: Color32,
    #[builder(default = Stroke::NONE)]
    background_stroke: Stroke,
    #[builder(default = Rounding::none())]
    background_rounding: Rounding,
}

impl Widget for MarginContainer {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let widget_id = self.id.resolve(parent_id);

        let mut content_layout =
            self.contents
                .widget
                .layout(ctx, widget_id, available - self.margin, force_shrink);
        content_layout.translate(self.margin * 0.5);
        Layout::with_children(
            widget_id,
            content_layout.bounds.size() + self.margin,
            vec![content_layout],
        )
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        ctx.painter().rect(RectShape {
            rect: layout.bounds,
            rounding: self.background_rounding,
            fill: self.background_color,
            stroke: self.background_stroke,
        });

        self.contents.widget.draw(ctx, &layout.children[0])
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
            .on_event(ctx, &layout.children[0], cursor_position, events, status)
    }
}
