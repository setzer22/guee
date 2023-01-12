use crate::{
    context::Context,
    input::{Event, EventStatus, MouseButton},
    layout::{Layout, LayoutHints, SizeHint},
    widget::{DynWidget, Widget},
};
use epaint::{Color32, Pos2, RectShape, Rounding, Shape, Stroke, Vec2};
use typed_builder::TypedBuilder;

#[derive(Clone, TypedBuilder)]
pub struct Button {
    #[builder(default, setter(skip))]
    pub pressed: bool,
    #[builder(default, setter(skip))]
    pub hovered: bool,
    #[builder(default)]
    pub hints: LayoutHints,
    #[builder(default = Vec2::new(10.0, 10.0))]
    pub padding: Vec2,
    pub contents: DynWidget,
}

impl Widget for Button {
    fn layout(&mut self, ctx: &Context, available: Vec2) -> Layout {
        let padding = self.padding;
        let mut contents_layout = self.contents.widget.layout(ctx, available - padding);

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

        Layout::with_children(Vec2::new(width, height), vec![contents_layout])
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        ctx.shapes.borrow_mut().push(Shape::Rect(RectShape {
            rect: layout.bounds,
            rounding: Rounding::same(2.0),
            fill: Color32::from_rgba_unmultiplied(40, 200, 40, 50),
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

    fn on_event(&mut self, layout: &Layout, cursor_position: Pos2, event: &Event) -> EventStatus {
        if layout.bounds.contains(cursor_position) {
            self.hovered = true;
        }
        match event {
            Event::MousePressed(MouseButton::Primary) => {
                self.pressed = true;
                return EventStatus::Consumed;
            }
            _ => {}
        }

        EventStatus::Ignored
    }
}
