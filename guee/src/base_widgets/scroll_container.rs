use epaint::{RectShape, Rounding};
use guee_derives::Builder;

use crate::{input::MouseButton, painter::TranslateScale, prelude::*};

#[derive(Builder)]
#[builder(widget)]
pub struct VScrollContainer {
    pub id: IdGen,
    pub contents: DynWidget,
    #[builder(default)]
    pub hints: LayoutHints,
    #[builder(default)]
    pub min_height: f32,
    #[builder(default = 16.0)]
    pub scrollbar_size: f32,
}

pub struct VScrollContainerState {
    // Scrollbar position, between 1 and 0
    pub scrollbar_frac: f32,
}

impl VScrollContainer {
    pub fn y_offset(&self, layout: &Layout, scrollbar_frac: f32) -> f32 {
        (layout.children[0].bounds.height() - layout.bounds.height()) * scrollbar_frac
    }

    pub fn scrollbar_handle_bounds(&self, layout: &Layout, scrollbar_frac: f32) -> Rect {
        let scrollbar = layout.children[1].bounds;
        let handle_height =
            scrollbar.height() * (layout.bounds.height() / layout.children[0].bounds.height());
        let handle_pos = (scrollbar.height() - handle_height) * scrollbar_frac;

        Rect::from_min_size(
            Pos2::new(scrollbar.left(), scrollbar.top() + handle_pos),
            Vec2::new(self.scrollbar_size, handle_height),
        )
        // TODO: Theme
        .shrink2(Vec2::new(2.0, 2.0))
    }
}

impl Widget for VScrollContainer {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let widget_id = self.id.resolve(parent_id);

        let shrink_ch_layout = self.contents.widget.layout(ctx, parent_id, available, true);

        let width = match self.hints.size_hints.width.or_force(force_shrink) {
            SizeHint::Shrink => shrink_ch_layout.bounds.width() + self.scrollbar_size,
            SizeHint::Fill => available.x,
        };

        let height = match self.hints.size_hints.height.or_force(force_shrink) {
            SizeHint::Shrink => self.min_height,
            SizeHint::Fill => available.y,
        };

        let ch_layout = self.contents.widget.layout(
            ctx,
            parent_id,
            Vec2::new(width - self.scrollbar_size, height),
            force_shrink,
        );

        let scrollbar_pos = ch_layout.bounds.right_top();
        let scrollbar_size = Vec2::new(self.scrollbar_size, height);
        let scrollbar_layout = Layout::leaf(widget_id.with("scrollbar"), scrollbar_size)
            .translated(scrollbar_pos.to_vec2());

        Layout::with_children(
            widget_id,
            Vec2::new(width, height),
            vec![ch_layout, scrollbar_layout],
        )
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let scrollbar_frac = ctx
            .memory
            .get::<VScrollContainerState>(layout.widget_id)
            .scrollbar_frac;
        let y_offset = self.y_offset(layout, scrollbar_frac);

        let old_transform = ctx.painter().transform;
        let old_clip_rect = ctx.painter().clip_rect;

        ctx.painter().transform = old_transform.translated(-Vec2::Y * y_offset);
        ctx.painter().clip_rect = layout.bounds;

        self.contents.widget.draw(ctx, &layout.children[0]);

        ctx.painter().transform = old_transform;
        ctx.painter().clip_rect = old_clip_rect;

        let scrollbar_rect = layout.children[1].bounds;
        ctx.painter().rect(RectShape {
            rect: scrollbar_rect,
            rounding: Rounding::none(),
            fill: color!("#191919"), // TODO Theme
            stroke: Stroke::NONE,
        });

        ctx.painter().rect(RectShape {
            rect: self.scrollbar_handle_bounds(layout, scrollbar_frac),
            rounding: Rounding::same(1.0),
            fill: color!("#303030"),
            stroke: Stroke::new(1.0, color!("#464646")),
        })
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
        let scrollbar_frac = ctx
            .memory
            .get_or::<VScrollContainerState>(
                layout.widget_id,
                VScrollContainerState {
                    scrollbar_frac: 0.0,
                },
            )
            .scrollbar_frac;

        // Set cursor transform
        let cursor_transform =
            TranslateScale::identity().translated(Vec2::Y * self.y_offset(layout, scrollbar_frac));
        let ch_status = ctx.with_cursor_transform(cursor_transform, || {
            let transformed_cursor_position = cursor_transform.transform_point(cursor_position);
            self.contents.widget.on_event(
                ctx,
                &layout.children[0],
                transformed_cursor_position,
                events,
            )
        });

        if ch_status == EventStatus::Consumed {
            return EventStatus::Consumed;
        }

        let mut state = ctx
            .memory
            .get_mut::<VScrollContainerState>(layout.widget_id);
        let mut status = EventStatus::Ignored;
        if layout.bounds.contains(cursor_position) {
            for event in events {
                if let Event::MouseWheel(delta) = &event {
                    state.scrollbar_frac = (state.scrollbar_frac - delta.y * 0.05).clamp(0.0, 1.0);
                    status = EventStatus::Consumed;
                }
            }
        }

        let handle_bounds = self.scrollbar_handle_bounds(layout, scrollbar_frac);
        if ctx.claim_drag_event(layout.widget_id, handle_bounds, MouseButton::Primary) {
            let delta = ctx.input_state.mouse.delta().y;
            let main_size = layout.bounds.height() - handle_bounds.height();
            state.scrollbar_frac += delta / main_size;
            state.scrollbar_frac = state.scrollbar_frac.clamp(0.00, 1.0);
            status = EventStatus::Consumed;
        }

        status
    }
}
