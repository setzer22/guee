use std::any::{Any, TypeId};

use epaint::{ahash::HashMap, Pos2, Vec2};
use guee_derives::Builder;

use crate::{extension_traits::Vec2Ext, input::MouseButton, prelude::*};

#[derive(Builder)]
#[builder(widget, skip_new)]
pub struct DragValue {
    pub value: f32,
    #[builder(callback)]
    pub on_changed: Option<Callback<f32>>,

    #[builder(skip)]
    pub text_edit: TextEdit,
}

pub struct DragValueState {
    pub last_focus_state: bool,
    pub string_contents: String,
    pub acc_drag: Vec2,
}

impl DragValue {
    pub fn format_contents(contents: f32) -> String {
        format!("{contents:.4}")
    }

    pub fn contents_from_string(s: &str) -> Option<f32> {
        s.parse().ok()
    }

    pub fn new(id: IdGen, value: f32) -> Self {
        DragValue {
            value,
            on_changed: None,
            // The string is patched later, during `Layout`, depending on this
            // widget's internal state.
            text_edit: TextEdit::new(id, String::new()),
        }
    }

    // TODO: Make #[derive(Builder)] capable of forwarding builder functions to
    // some of the fields
    pub fn layout_hints(mut self, layout_hints: LayoutHints) -> Self {
        self.text_edit = self.text_edit.layout_hints(layout_hints);
        self
    }

    // TODO: Make #[derive(Builder)] capable of forwarding builder functions to
    // some of the fields
    pub fn padding(mut self, padding: Vec2) -> Self {
        self.text_edit = self.text_edit.padding(padding);
        self
    }
}

impl Widget for DragValue {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let widget_id = self.text_edit.id.resolve(parent_id);
        let is_focused = ctx.is_focused(widget_id);
        // TODO Nitpick: Add get_or_else so we don't have to allocate twice
        let state = ctx.memory.get_or(
            widget_id,
            DragValueState {
                last_focus_state: is_focused,
                string_contents: Self::format_contents(self.value),
                acc_drag: Vec2::ZERO,
            },
        );

        if is_focused {
            self.text_edit.contents = state.string_contents.clone();
        } else {
            self.text_edit.contents = Self::format_contents(self.value);
        }

        drop(state);

        let layout = self
            .text_edit
            .layout(ctx, parent_id, available, force_shrink);
        // Check invariants, just in case...
        assert!(
            layout.widget_id == widget_id,
            "Child widget should have the same id as we assumed"
        );
        layout
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        self.text_edit.draw(ctx, layout)
    }

    fn layout_hints(&self) -> LayoutHints {
        let text_edit: &dyn Widget = &self.text_edit;
        text_edit.layout_hints()
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        // A drag event will engage "drag" mode, while a click event will focus
        // and toggle the inner TextEdit.
        let dragging = ctx
            .claim_drag_event(layout.widget_id, layout.bounds, MouseButton::Primary)
            .is_some();

        // A TextEdit normally focuses itself, but we are inhibiting that below
        // by not feeding it events unless it's focused.
        if layout.bounds.contains(cursor_position)
            && ctx
                .input_state
                .mouse_state
                .button_state
                .is_clicked(MouseButton::Primary)
        {
            ctx.request_focus(layout.widget_id);
        }

        let focused_now = ctx.is_focused(layout.widget_id);

        // Set up internal callback so we can get the result from on_changed and
        // transform the value
        let (cb, tk) = ctx.create_internal_callback();
        self.text_edit.on_changed = Some(cb);

        // If the child is not focused, ignore its event processing logic
        // We instead do our own focus handling
        let mut status = self.text_edit.on_event(
            ctx,
            layout,
            cursor_position,
            if focused_now { events } else { &[] },
        );

        let mut state = ctx.memory.get_mut::<DragValueState>(layout.widget_id);

        // Check if the component was just focused during this frame
        let just_focused = state.last_focus_state != focused_now && focused_now;
        state.last_focus_state = focused_now;

        if just_focused {
            // When first focused, the string contents are overriden with
            // whatever float value we have, so that when the editor gains focus
            // the string is like the user was seeing it in the UI. Displaying
            // the old value can lead to confusing results.
            state.string_contents = Self::format_contents(self.value);
        }

        // When the TextEdit is focused, it should behave like a regular
        // TextEdit, letting the user write anything in the text box
        if focused_now {
            if let Some(result) = ctx.poll_callback_result(tk) {
                // If the inner text changed, replace the contents in transient state
                state.string_contents = result.clone();
                status = EventStatus::Consumed;

                // Additionally, if the contents can be parsed as float, emit
                // our on_changed event
                if let Some(new_value) = Self::contents_from_string(&result) {
                    if let Some(on_changed) = self.on_changed.take() {
                        ctx.dispatch_callback(on_changed, new_value);
                    }
                }
            }
        } else if dragging {
            const MOUSE_AIM_PRECISION: f32 = 20.0;
            const SCROLL_WHEEL_PRECISION: f32 = 50.0;

            let ctrl_held: bool = false;

            if ctrl_held {
                // TODO: Do we need to handle scale in the delta?
                state.acc_drag += ctx.input_state.mouse_state.delta().y * Vec2::Y;
            } else {
                state.acc_drag += ctx.input_state.mouse_state.delta().x * Vec2::X;
            }

            let discrete_increments = (state.acc_drag / MOUSE_AIM_PRECISION).floor();
            state.acc_drag = state.acc_drag.rem_euclid(MOUSE_AIM_PRECISION);

            let delta_value = discrete_increments.x * 0.1;
            let new_value = self.value + delta_value;
            if let Some(on_changed) = self.on_changed.take() {
                ctx.dispatch_callback(on_changed, new_value);
                status = EventStatus::Consumed
            }
        }

        status
    }
}
