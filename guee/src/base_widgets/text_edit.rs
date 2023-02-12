use epaint::{
    text::cursor::Cursor, Color32, FontId, Pos2, RectShape, Rounding, Stroke,
    Vec2,
};
use guee_derives::Builder;
use winit::event::VirtualKeyCode;

use crate::{
    callback::Callback,
    context::Context,
    input::{Event, EventStatus, MouseButton},
    layout::{Layout, LayoutHints, SizeHint},
    painter::{GueeGalley, GueeTextShape},
    widget::Widget,
    widget_id::{IdGen, WidgetId},
};

use super::button::Button;

pub mod text_buffer;

#[derive(Builder)]
#[builder(widget)]
pub struct TextEdit {
    id: IdGen,
    contents: String,
    #[builder(default)]
    padding: Vec2,
    #[builder(default)]
    layout_hints: LayoutHints,
    #[builder(skip)]
    galley: Option<GueeGalley>,
    #[builder(callback)]
    on_changed: Option<Callback<String>>,
    #[builder(default = 100.0)]
    min_width: f32,
}

#[derive(Default)]
pub struct TextEditUiState {
    cursor: Cursor,
}

impl Widget for TextEdit {
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        let widget_id = self.id.resolve(parent_id);
        let padding = self.padding;

        let size_hints = self.layout_hints.size_hints;
        let width = match size_hints.width {
            SizeHint::Shrink => self.min_width + 2.0 * padding.x,
            SizeHint::Fill => available.x,
        };

        let galley = ctx
            .painter()
            .galley(self.contents.clone(), FontId::proportional(14.0), width);
        self.galley = Some(galley.clone());

        let height = match size_hints.height {
            SizeHint::Shrink => galley.bounds().width() + 2.0 * padding.y,
            SizeHint::Fill => available.y,
        };

        Layout::leaf(widget_id, Vec2::new(width, height))
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let ui_state = ctx
            .memory
            .get_mut_or(layout.widget_id, TextEditUiState::default());
        let focused = ctx.is_focused(layout.widget_id);

        ctx.painter().rect(RectShape {
            rect: layout.bounds,
            rounding: Rounding::same(1.0),
            fill: Color32::from_rgb(40, 40, 40),
            stroke: Stroke::new(2.0, Color32::from_rgb(80, 80, 80)),
        });

        let text_bounds = layout.bounds.shrink2(self.padding);

        let galley = self.galley.clone().unwrap();
        ctx.painter().text(GueeTextShape {
            pos: text_bounds.left_top(),
            galley: galley.clone(),
            underline: Stroke::NONE,
            angle: 0.0,
        });

        if focused {
            let cursor = galley.epaint_galley.cursor_end_of_row(&ui_state.cursor);
            let cursor_rect = galley.epaint_galley
                .pos_from_cursor(&cursor)
                .expand2(Vec2::new(1.0, 0.0))
                .translate(text_bounds.left_top().to_vec2());
            ctx.painter().rect(RectShape {
                rect: cursor_rect,
                rounding: Rounding::none(),
                fill: Color32::WHITE,
                stroke: Stroke::NONE,
            });
        }
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
        events: &[Event],
    ) -> EventStatus {
        let mut _ui_state = ctx
            .memory
            .get_mut_or(layout.widget_id, TextEditUiState::default());
        let is_focused = ctx.is_focused(layout.widget_id);
        let _galley = self.galley.as_ref().unwrap();

        for event in events {
            match event {
                Event::MousePressed(MouseButton::Primary) => {
                    if layout.bounds.contains(cursor_position) {
                        ctx.request_focus(layout.widget_id);
                    }
                }
                Event::Text(ch) if is_focused => {
                    let mut contents = self.contents.clone();
                    contents.push(*ch);
                    ctx.dispatch_callback(self.on_changed.take().unwrap(), contents);
                }
                Event::KeyPressed(VirtualKeyCode::Back) if is_focused => {
                    if !self.contents.is_empty() {
                        let mut contents = self.contents.clone();
                        contents.drain(self.contents.len() - 1..);
                        ctx.dispatch_callback(self.on_changed.take().unwrap(), contents);
                    }
                }
                _ => {}
            }
        }

        EventStatus::Ignored
    }
}
