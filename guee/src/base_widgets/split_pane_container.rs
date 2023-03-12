use std::{any::type_name, ops::DerefMut};

use epaint::{Color32, Pos2, Rect, RectShape, Rounding, Stroke, Vec2};
use guee_derives::Builder;

use crate::{
    context::Context,
    input::{Event, EventStatus, MouseButton},
    layout::{Layout, LayoutHints},
    prelude::{Axis, AxisDirections, SizeHint, StyledWidget},
    widget::{DynWidget, Widget},
    widget_id::{IdGen, WidgetId},
};

#[derive(Builder)]
#[builder(widget)]
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
}

#[derive(Builder)]
pub struct SplitPaneContainerStyle {
    pub handle_color: Color32,
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
        .translate(bounds.left_top().to_vec2())
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
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool, // ignored, always expanded.
    ) -> Layout {
        if force_shrink {
            SizeHint::ignore_force_warning(type_name::<Self>());
        }

        let widget_id = self.id.resolve(parent_id);
        let axis = self.axis;
        let frac = self.get_frac(widget_id, ctx);

        let handle = axis.new_vec2(self.handle_width, 0.0);

        let available_left = axis.vec2_scale(available, frac, 1.0) - handle;
        let available_right = axis.vec2_scale(available, 1.0 - frac, 1.0) - handle;

        let left_layout = self
            .left_widget
            .widget
            .layout(ctx, widget_id, available_left, false);

        let offset = available.main_dir(axis) * frac + self.handle_width;
        let right_layout = self
            .right_widget
            .widget
            .layout(ctx, widget_id, available_right, false)
            .translated(axis.new_vec2(offset, 0.0));

        Layout::with_children(widget_id, available, vec![left_layout, right_layout])
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let frac = self.get_frac(layout.widget_id, ctx);
        self.left_widget.widget.draw(ctx, &layout.children[0]);
        self.right_widget.widget.draw(ctx, &layout.children[1]);

        let default_style = SplitPaneContainerStyle {
            handle_color: Color32::BLACK,
        };
        let theme = ctx.theme.borrow();
        let style = theme.get_style::<Self>().unwrap_or(&default_style);

        if self.hovered {
            let handle_rect = self.resize_handle_visual_rect(frac, layout.bounds);
            ctx.painter().rect(RectShape {
                rect: handle_rect,
                rounding: Rounding::same(2.0),
                fill: style.handle_color,
                stroke: Stroke::NONE,
            });
        }
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
        status: &mut EventStatus,
    ) {
        if !status.is_consumed() {
            let mut state = self.get_mut_state(layout.widget_id, ctx);

            let handle_rect = self
                .resize_handle_rect(state.frac, layout.bounds)
                // Make it easier to interact with
                .expand2(self.axis.new_vec2(5.0, 0.0));

            let mut status = EventStatus::Ignored;

            if handle_rect.contains(cursor_position) {
                self.hovered = true;
            }

            if ctx.claim_drag_event(layout.widget_id, handle_rect, MouseButton::Primary) {
                let delta = ctx.input_state.mouse.delta().main_dir(self.axis);
                let main_size = layout.bounds.size().main_dir(self.axis);
                state.frac += delta / main_size;
                state.frac = state.frac.clamp(0.01, 0.99);
                // Prevents hovering other widgets while dragging
                self.hovered = true;
                status.consume_event();
            }
        }

        self.left_widget
            .widget
            .on_event(ctx, &layout.children[0], cursor_position, events, status);

        self.right_widget.widget.on_event(
            ctx,
            &layout.children[1],
            cursor_position,
            events,
            status,
        );
    }
}

impl StyledWidget for SplitPaneContainer {
    type Style = SplitPaneContainerStyle;
}
