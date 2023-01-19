use std::sync::Arc;

use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints, SizeHint, SizeHints},
    widget::Widget,
    widget_id::WidgetId,
};
use epaint::{Color32, FontId, Fonts, Galley, Pos2, Shape, Stroke, TextShape, Vec2};
use guee_derives::Builder;
use typed_builder::TypedBuilder;

#[derive(Clone, Builder)]
pub struct Text {
    contents: String,
    #[builder(skip)]
    last_galley: Option<Arc<Galley>>,
}

impl Text {
    pub fn ensure_galley(&mut self, fonts: &Fonts, wrap_width: f32) -> Arc<Galley> {
        let galley = fonts.layout(
            self.contents.clone(),
            FontId::proportional(14.0),
            Color32::BLACK,
            wrap_width,
        );
        self.last_galley = Some(galley.clone());
        galley
    }
}

impl Widget for Text {
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        Layout::leaf(
            parent_id.with(&self.contents),
            self.min_size(ctx, available),
        )
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let galley = self
            .last_galley
            .clone()
            .expect("Layout should be called before draw");
        ctx.shapes.borrow_mut().push(Shape::Text(TextShape {
            pos: layout.bounds.left_top(),
            galley,
            underline: Stroke::NONE,
            override_text_color: None,
            angle: 0.0,
        }));
    }

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        let galley = self.ensure_galley(&ctx.fonts, available.x);
        galley.rect.size()
    }

    fn layout_hints(&self) -> LayoutHints {
        LayoutHints {
            size_hints: SizeHints {
                width: SizeHint::Shrink,
                height: SizeHint::Shrink,
            },
            weight: 1,
        }
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        EventStatus::Ignored
    }
}
