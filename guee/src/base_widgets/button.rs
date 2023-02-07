use crate::{
    callback::Callback,
    context::Context,
    extension_traits::Color32Ext,
    input::{Event, EventStatus, MouseButton},
    layout::{Layout, LayoutHints, SizeHint},
    prelude::StyledWidget,
    widget::{DynWidget, Widget},
    widget_id::{IdGen, WidgetId},
};
use epaint::{Color32, Pos2, RectShape, Rounding, Shape, Stroke, Vec2};
use guee_derives::Builder;

use super::text::Text;

#[derive(Builder)]
#[builder(widget)]
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
    pub on_click: Option<Callback<()>>,
}

#[derive(Builder, Default)]
pub struct ButtonStyle {
    pub pressed_fill: Color32,
    pub pressed_stroke: Stroke,
    pub hovered_fill: Color32,
    pub hovered_stroke: Stroke,
    pub idle_fill: Color32,
    pub idle_stroke: Stroke,
    #[builder(default = Color32::BLACK)]
    pub text_color: Color32,
    #[builder(default = Rounding::same(2.0))]
    pub rounding: Rounding,
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
        let default_style = ButtonStyle::default();
        let theme = ctx.theme.borrow();
        let style = theme.get_style::<Self>().unwrap_or(&default_style);

        ctx.shapes.borrow_mut().push(Shape::Rect(RectShape {
            rect: layout.bounds,
            rounding: style.rounding,
            fill: if self.pressed {
                style.pressed_fill
            } else if self.hovered {
                style.hovered_fill
            } else {
                style.idle_fill
            },
            stroke: if self.pressed {
                style.pressed_stroke
            } else if self.hovered {
                style.hovered_stroke
            } else {
                style.idle_stroke
            },
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
        events: &[Event],
    ) -> EventStatus {
        if layout.bounds.contains(cursor_position) {
            self.hovered = true;
            for event in events {
                if let Event::MousePressed(MouseButton::Primary) = event {
                    if let Some(on_click) = self.on_click.take() {
                        ctx.dispatch_callback(on_click, ())
                    }
                    self.pressed = true;
                    return EventStatus::Consumed;
                }
            }
        }

        EventStatus::Ignored
    }
}

impl ButtonStyle {
    pub fn with_base_colors(
        fill: Color32,
        stroke: Stroke,
        hover_mul: f32,
        pressed_mul: f32,
    ) -> Self {
        Self::new(
            fill.lighten(pressed_mul),
            stroke.lighten(pressed_mul),
            fill.lighten(hover_mul),
            stroke.lighten(hover_mul),
            fill,
            stroke,
        )
    }
}

impl StyledWidget for Button {
    type Style = ButtonStyle;
}
