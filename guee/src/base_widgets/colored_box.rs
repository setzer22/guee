use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints, SizeHint},
    widget::Widget,
    widget_id::{IdGen, WidgetId},
};
use epaint::{Color32, Pos2, RectShape, Rounding, Stroke, Vec2};
use guee_derives::Builder;

#[derive(Builder)]
#[builder(widget)]
pub struct ColoredBox {
    pub id: IdGen,
    #[builder(default)]
    pub hints: LayoutHints,
    #[builder(default)]
    pub min_size: Vec2,
    #[builder(default)]
    pub rounding: Rounding,
    #[builder(default)]
    pub fill: Color32,
    #[builder(default)]
    pub stroke: Stroke,
}

impl ColoredBox {
    pub fn background(color: Color32) -> Self {
        Self::new(IdGen::key("background"))
            .hints(LayoutHints::fill())
            .fill(color)
    }
}

impl Widget for ColoredBox {
    fn layout(&mut self, _ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        let widget_id = self.id.resolve(parent_id);
        let size_hints = self.hints.size_hints;
        let width = match size_hints.width {
            SizeHint::Shrink => self.min_size.x,
            SizeHint::Fill => available.x,
        };
        let height = match size_hints.height {
            SizeHint::Shrink => self.min_size.y,
            SizeHint::Fill => available.y,
        };

        Layout::leaf(widget_id, Vec2::new(width, height))
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        ctx.painter().rect(RectShape {
            rect: layout.bounds,
            rounding: self.rounding,
            fill: self.fill,
            stroke: self.stroke,
        });
    }

    fn min_size(&mut self, _ctx: &Context, _available: Vec2) -> Vec2 {
        self.min_size
    }

    fn layout_hints(&self) -> LayoutHints {
        self.hints
    }

    fn on_event(
        &mut self,
        _ctx: &Context,
        _layout: &Layout,
        _cursor_position: Pos2,
        _events: &[Event],
    ) -> EventStatus {
        EventStatus::Ignored
    }
}
