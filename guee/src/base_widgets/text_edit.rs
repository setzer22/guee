use epaint::{text::cursor::Cursor, Color32, FontId, Pos2, RectShape, Rounding, Stroke, Vec2};
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

pub mod text_buffer;

#[derive(Builder)]
#[builder(widget)]
pub struct TextEdit {
    pub id: IdGen,
    pub contents: String,
    #[builder(default = Vec2::new(3.0, 0.0))]
    pub padding: Vec2,
    #[builder(default)]
    pub layout_hints: LayoutHints,
    #[builder(skip)]
    pub galley: Option<GueeGalley>,
    #[builder(strip_option)]
    pub on_changed: Option<Callback<String>>,
    #[builder(default = 60.0)]
    pub min_width: f32,
    #[builder(default = 14.0)]
    pub font_size: f32,
}

#[derive(Default)]
pub struct TextEditUiState {
    cursor: Cursor,
}

impl Widget for TextEdit {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let widget_id = self.id.resolve(parent_id);
        let padding = self.padding;

        let size_hints = self.layout_hints.size_hints;
        let width = match size_hints.width.or_force(force_shrink) {
            SizeHint::Shrink => self.min_width + 2.0 * padding.x,
            SizeHint::Fill => available.x,
        };

        let galley = ctx.painter().galley(
            self.contents.clone(),
            FontId::proportional(self.font_size),
            // The text in a text edit does not wrap at a certain width.
            f32::INFINITY,
        );
        self.galley = Some(galley.clone());

        let height = match size_hints.height {
            SizeHint::Shrink => galley.bounds().height() + 2.0 * padding.y,
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
        ctx.painter().text_with_galley(GueeTextShape {
            pos: text_bounds.left_top(),
            galley: galley.clone(),
            underline: Stroke::NONE,
            angle: 0.0,
        });

        if focused {
            let cursor = galley.epaint_galley.cursor_end_of_row(&ui_state.cursor);
            let cursor_rect = galley
                .epaint_galley
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
        let cursor_in_bounds = layout.bounds.contains(cursor_position);
        let _galley = self.galley.as_ref().unwrap();

        let mut event_status = EventStatus::Ignored;

        for event in events {
            match event {
                Event::MousePressed(MouseButton::Primary) if cursor_in_bounds => {
                    ctx.request_focus(layout.widget_id);
                    event_status = EventStatus::Consumed;
                }
                Event::Text(ch) if is_focused => {
                    let mut contents = self.contents.clone();
                    contents.push(*ch);
                    if let Some(on_changed) = self.on_changed.take() {
                        ctx.dispatch_callback(on_changed, contents);
                    }
                    event_status = EventStatus::Consumed;
                }
                Event::KeyPressed(VirtualKeyCode::Back) if is_focused => {
                    if !self.contents.is_empty() {
                        let mut contents = self.contents.clone();
                        contents.drain(self.contents.len() - 1..);
                        if let Some(on_changed) = self.on_changed.take() {
                            ctx.dispatch_callback(on_changed, contents);
                        }
                    }
                    event_status = EventStatus::Consumed;
                }
                Event::KeyPressed(VirtualKeyCode::Escape) if is_focused => {
                    ctx.release_focus(layout.widget_id);
                }
                _ => {}
            }
        }

        event_status
    }
}
