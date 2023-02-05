use std::ops::DerefMut;

use epaint::{Color32, Pos2, Rect, RectShape, Rounding, Stroke, Vec2};
use guee_derives::Builder;

use crate::{
    context::Context,
    input::{Event, EventStatus, MouseButton},
    layout::{Layout, LayoutHints},
    prelude::{Axis, AxisDirections},
    widget::{DynWidget, Widget},
    widget_id::{IdGen, WidgetId},
};

#[derive(Builder)]
pub struct SplitPaneContainer {
    id: IdGen,
    #[builder(default)]
    margin: Vec2,
    axis: Axis,
    left_widget: DynWidget,
    right_widget: DynWidget,
    #[builder(default = 0.50)]
    default_frac: f32,
    #[builder(default = 4.0)]
    handle_width: f32,
    #[builder(skip)]
    hovered: bool,
    #[builder(skip)]
    clicked: bool,
}

pub struct SplitPaneContainerState {
    frac: f32,
}

impl SplitPaneContainer {
    pub fn resize_handle_rect(&self, frac: f32, bounds: Rect) -> Rect {
        let main_size = bounds.size().main_dir(self.axis);
        let main_center = main_size * frac;
        let cross_size = bounds.size().cross_dir(self.axis);
        let cross_center = cross_size * 0.5;
        Rect::from_center_size(
            self.axis.new_vec2(main_center, cross_center).to_pos2(),
            self.axis.new_vec2(self.handle_width, cross_size),
        )
    }
    pub fn resize_handle_visual_rect(&self, frac: f32, bounds: Rect) -> Rect {
        let handle_rect = self.resize_handle_rect(frac, bounds);
        handle_rect.shrink2(self.axis.new_vec2(0.5, 0.90))
    }

    pub fn get_frac(&self, widget_id: WidgetId, ctx: &Context) -> f32 {
        ctx.memory
            .get_or(
                widget_id,
                SplitPaneContainerState {
                    frac: self.default_frac,
                },
            )
            .frac
    }

    pub fn get_mut_state<'ctx>(
        &self,
        widget_id: WidgetId,
        ctx: &'ctx Context,
    ) -> impl DerefMut<Target = SplitPaneContainerState> + 'ctx {
        ctx.memory.get_mut(widget_id)
    }
}

impl Widget for SplitPaneContainer {
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        let widget_id = self.id.resolve(parent_id);
        let axis = self.axis;
        let frac = self.get_frac(widget_id, ctx);

        let handle = axis.new_vec2(self.handle_width, 0.0);

        let available_left = axis.vec2_scale(available, frac, 1.0) - handle;
        let available_right = axis.vec2_scale(available, 1.0 - frac, 1.0) - handle;

        let left_layout = self
            .left_widget
            .widget
            .layout(ctx, widget_id, available_left);

        let offset = available.main_dir(axis) * frac + self.handle_width;
        let right_layout = self
            .right_widget
            .widget
            .layout(ctx, widget_id, available_right)
            .translated(axis.new_vec2(offset, 0.0));

        Layout::with_children(widget_id, available, vec![left_layout, right_layout])
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let frac = self.get_frac(layout.widget_id, ctx);
        self.left_widget.widget.draw(ctx, &layout.children[0]);
        self.right_widget.widget.draw(ctx, &layout.children[1]);
        if self.hovered {
            let handle_rect = self.resize_handle_visual_rect(frac, layout.bounds);
            ctx.shapes.borrow_mut().push(epaint::Shape::Rect(RectShape {
                rect: handle_rect,
                rounding: Rounding::same(2.0),
                fill: Color32::GREEN,
                stroke: Stroke::NONE,
            }));
        }
    }

    fn min_size(&mut self, _ctx: &Context, available: Vec2) -> Vec2 {
        available
    }

    fn layout_hints(&self) -> LayoutHints {
        // NOTE: This widget does not allow configurable hints. It is always
        // fully expanded.
        LayoutHints::fill()
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        let mut state = self.get_mut_state(layout.widget_id, ctx);

        let mouse_in_handle = self
            .resize_handle_rect(state.frac, layout.bounds)
            .contains(cursor_position);

        if mouse_in_handle {
            // Try to handle events in resize area
            let mut status = EventStatus::Ignored;
            if ctx
                .input_state
                .mouse_state
                .button_state
                .is_down(MouseButton::Primary)
            {
                self.clicked = true;
                status = EventStatus::Consumed;
                let delta = ctx.input_state.mouse_state.delta().main_dir(self.axis);
                // WIP: This delta is clearly wrong. I need to convert from
                // pixels to fraction using the rect bounds size.
                state.frac += delta * 0.01;
            };
            self.hovered = true;
            status
        } else {
            // Handle events in children
            if self
                .left_widget
                .widget
                .on_event(ctx, &layout.children[0], cursor_position, events)
                == EventStatus::Consumed
            {
                EventStatus::Consumed
            } else {
                self.right_widget
                    .widget
                    .on_event(ctx, &layout.children[1], cursor_position, events)
            }
        }
    }
}
