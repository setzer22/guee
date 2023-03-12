use crate::prelude::*;
use epaint::{Pos2, Vec2};
use guee_derives::Builder;

#[derive(Builder)]
#[builder(widget)]
pub struct Image {
    pub id: IdGen,
    pub texture_id: TextureId,
    pub hints: LayoutHints,
    #[builder(default)]
    pub min_size: Vec2,
}

impl Widget for Image {
    fn layout(
        &mut self,
        _ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let widget_id = self.id.resolve(parent_id);
        let size_hints = self.hints.size_hints;
        let width = match size_hints.width.or_force(force_shrink) {
            SizeHint::Shrink => self.min_size.x,
            SizeHint::Fill => available.x,
        };
        let height = match size_hints.height.or_force(force_shrink) {
            SizeHint::Shrink => self.min_size.y,
            SizeHint::Fill => available.y,
        };
        Layout::leaf(widget_id, Vec2::new(width, height))
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        ctx.painter().image(layout.bounds, self.texture_id);
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
        _event_status: &mut EventStatus,
    ) {
    }
}
