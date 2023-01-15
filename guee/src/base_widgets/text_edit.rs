use std::sync::Arc;

use epaint::{
    text::cursor::Cursor, Color32, FontId, Galley, Pos2, RectShape, Rounding, Shape, Stroke,
    TextShape, Vec2,
};
use guee_derives::Builder;

use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints},
    widget::Widget,
    widget_id::{IdGen, WidgetId},
};

use self::text_buffer::TextBuffer;

use super::button::Button;

pub mod text_buffer;

#[derive(Builder)]
pub struct TextEdit {
    id: IdGen,
    contents: String,
    #[builder(default)]
    padding: Vec2,
    #[builder(default)]
    layout_hints: LayoutHints,
    #[builder(skip)]
    galley: Option<Arc<Galley>>,
}

#[derive(Default)]
pub struct TextEditUiState {
    cursor: Cursor,
}

impl Widget for TextEdit {
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        let widget_id = self.id.resolve(parent_id);
        // Delegate layouting to button, since the two widgets are very similar
        let mut b = Button::with_label(self.contents.clone())
            .padding(self.padding)
            .hints(self.layout_hints);
        let mut b_layout = b.layout(ctx, widget_id, available);
        // Undo centering of inner text
        let text_left = b_layout.children[0].bounds.left();
        b_layout.children[0].translate_x(-text_left + self.padding.x);
        b_layout
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let ui_state = ctx.memory.get_mut_or(
            layout.widget_id,
            TextEditUiState {
                cursor: Cursor::default(),
            },
        );

        ctx.shapes.borrow_mut().push(Shape::Rect(RectShape {
            rect: layout.bounds,
            rounding: Rounding::same(1.0),
            fill: Color32::from_rgb(40, 40, 40),
            stroke: Stroke::new(2.0, Color32::from_rgb(80, 80, 80)),
        }));

        let text_bounds = layout.children[0].bounds;
        let galley = ctx.fonts.layout(
            self.contents.clone(),
            FontId::proportional(14.0),
            Color32::WHITE,
            text_bounds.size().x,
        );
        self.galley = Some(galley.clone());

        ctx.shapes.borrow_mut().push(Shape::Text(TextShape {
            pos: text_bounds.left_top(),
            galley: galley.clone(),
            underline: Stroke::NONE,
            override_text_color: None,
            angle: 0.0,
        }));

        let cursor = galley.cursor_end_of_row(&ui_state.cursor);

        let cursor_rect = galley
            .pos_from_cursor(&cursor)
            .expand2(Vec2::new(1.0, 0.0))
            .translate(text_bounds.left_top().to_vec2());
        ctx.shapes.borrow_mut().push(Shape::Rect(RectShape {
            rect: cursor_rect,
            rounding: Rounding::none(),
            fill: Color32::WHITE,
            stroke: Stroke::NONE,
        }));
    }

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        let mut b = Button::with_label(self.contents.clone())
            .padding(self.padding)
            .hints(self.layout_hints);
        b.min_size(ctx, available)
    }

    fn layout_hints(&self) -> LayoutHints {
        self.layout_hints
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        event: &Event,
    ) -> EventStatus {
        let ui_state = ctx.memory.get_mut_or(
            layout.widget_id,
            TextEditUiState {
                cursor: Cursor::default(),
            },
        );

        match event {
            Event::MousePressed(_) => {}
            _ => {}
        }

        EventStatus::Ignored
    }
}
