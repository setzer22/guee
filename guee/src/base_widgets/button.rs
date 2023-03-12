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
use epaint::{emath::Align2, Color32, Pos2, Rect, RectShape, Rounding, Stroke, Vec2};
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
    #[builder(default = Align2::CENTER_CENTER)]
    pub align_contents: Align2,
    pub contents: DynWidget,
    #[builder(strip_option)]
    pub on_click: Option<Callback<()>>,
    #[builder(default, strip_option)]
    pub style_override: Option<ButtonStyle>,
    #[builder(default)]
    pub min_size: Vec2,
}

#[derive(Builder, Default, Clone)]
pub struct ButtonStyle {
    pub pressed_fill: Color32,
    pub pressed_stroke: Stroke,
    pub hovered_fill: Color32,
    pub hovered_stroke: Stroke,
    pub idle_fill: Color32,
    pub idle_stroke: Stroke,
    #[builder(default = Rounding::same(2.0))]
    pub rounding: Rounding,
}

impl Button {
    pub fn with_label(label: impl Into<String>) -> Self {
        let label = label.into();
        Button::new(IdGen::key(&label), Text::new(label).build())
    }

    pub fn with_colored_label(label: impl Into<String>, color: Color32) -> Self {
        let label = label.into();
        Button::new(
            IdGen::key(&label),
            Text::new(label).color_override(color).build(),
        )
    }
}

impl Widget for Button {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let widget_id = self.id.resolve(parent_id);
        let padding = self.padding;
        let mut contents_layout =
            self.contents
                .widget
                .layout(ctx, widget_id, available - padding, force_shrink);

        let size_hints = self.hints.size_hints;
        let width = match size_hints.width.or_force(force_shrink) {
            SizeHint::Shrink => {
                contents_layout.bounds.width().max(self.min_size.x) + 2.0 * padding.x
            }
            SizeHint::Fill => available.x,
        };
        let height = match size_hints.height.or_force(force_shrink) {
            SizeHint::Shrink => {
                contents_layout.bounds.height().max(self.min_size.y) + 2.0 * padding.y
            }
            SizeHint::Fill => available.y,
        };

        contents_layout.bounds = self.align_contents.align_size_within_rect(
            contents_layout.bounds.size(),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height)).shrink2(self.padding),
        );

        Layout::with_children(widget_id, Vec2::new(width, height), vec![contents_layout])
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let default_style = ButtonStyle::default();
        let theme = ctx.theme.borrow();
        let style = self
            .style_override
            .as_ref()
            .unwrap_or_else(|| theme.get_style::<Self>().unwrap_or(&default_style));

        ctx.painter().rect(RectShape {
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
        });
        self.contents.widget.draw(ctx, &layout.children[0]);
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
        event_status: &mut EventStatus,
    ) {
        if event_status.is_consumed() {
            return;
        }

        if layout.bounds.contains(cursor_position) {
            self.hovered = true;
            for event in events {
                if let Event::MousePressed(MouseButton::Primary) = event {
                    if let Some(on_click) = self.on_click.take() {
                        ctx.dispatch_callback(on_click, ())
                    }
                    self.pressed = true;
                    *event_status = EventStatus::Consumed;
                }
            }
        }
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
