use crate::{
    callback::Callback,
    context::Context,
    input::{Event, EventStatus, MouseButton},
    layout::{Layout, LayoutHints, SizeHint},
    widget::{DynWidget, Widget},
    widget_id::{IdGen, WidgetId},
};
use epaint::{Color32, Pos2, RectShape, Rounding, Shape, Stroke, Vec2};
use guee_derives::Builder;

use super::text::Text;

#[derive(Builder)]
pub struct Button {
    pub id: IdGen,
    #[builder(skip)]
    pub pressed: bool,
    #[builder(skip)]
    pub hovered: bool,
    #[builder(default)]
    pub hints: LayoutHints,
    #[builder(default = Vec2::new(10.0, 10.0))]
    pub padding: Vec2,
    pub contents: DynWidget,
    #[builder(callback)]
    pub on_click: Option<Callback>,
}

impl Button {
    pub fn with_label(label: impl Into<String>) -> Self {
        let label = label.into();
        Button::new(IdGen::key(&label), Text::new(label).build())
    }
}

impl Widget for Button {
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        let widget_id = self.id.resolve(parent_id);
        let padding = self.padding;
        let mut contents_layout = self
            .contents
            .widget
            .layout(ctx, widget_id, available - padding);

        let size_hints = self.hints.size_hints;
        let width = match size_hints.width {
            SizeHint::Shrink => contents_layout.bounds.width() + 2.0 * padding.x,
            SizeHint::Fill => available.x,
        };
        let height = match size_hints.height {
            SizeHint::Shrink => contents_layout.bounds.height() + 2.0 * padding.y,
            SizeHint::Fill => available.y,
        };

        contents_layout.translate(Vec2::new(
            (width - contents_layout.bounds.width()) * 0.5,
            (height - contents_layout.bounds.height()) * 0.5,
        ));

        Layout::with_children(widget_id, Vec2::new(width, height), vec![contents_layout])
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        ctx.shapes.borrow_mut().push(Shape::Rect(RectShape {
            rect: layout.bounds,
            rounding: Rounding::same(2.0),
            fill: if self.pressed {
                Color32::from_rgba_unmultiplied(80, 240, 80, 50)
            } else if self.hovered {
                Color32::from_rgba_unmultiplied(50, 210, 50, 50)
            } else {
                Color32::from_rgba_unmultiplied(35, 195, 35, 50)
            },
            stroke: Stroke::NONE,
        }));
        self.contents.widget.draw(ctx, &layout.children[0]);
    }

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        self.contents
            .widget
            .min_size(ctx, available - self.padding * 2.0)
            + self.padding * 2.0
    }

    fn layout_hints(&self) -> LayoutHints {
        self.hints
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        event: &Event,
    ) -> EventStatus {
        if layout.bounds.contains(cursor_position) {
            self.hovered = true;
            match event {
                Event::MousePressed(MouseButton::Primary) => {
                    if let Some(on_click) = self.on_click.take() {
                        ctx.push_callback(on_click)
                    }
                    self.pressed = true;
                    return EventStatus::Consumed;
                }
                _ => {}
            }
        }

        EventStatus::Ignored
    }
}
