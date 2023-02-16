use std::any::{TypeId, Any};

use epaint::{Pos2, Vec2, ahash::HashMap};
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
    pub dragging: bool,
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
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout {
        let widget_id = self.text_edit.id.resolve(parent_id);
        let is_focused = ctx.is_focused(widget_id);
        // TODO Nitpick: Add get_or_else so we don't have to allocate twice
        let state = ctx.memory.get_or(
            widget_id,
            DragValueState {
                last_focus_state: is_focused,
                string_contents: Self::format_contents(self.value),
                acc_drag: Vec2::ZERO,
                dragging: false,
            },
        );

        if is_focused {
            self.text_edit.contents = state.string_contents.clone();
        } else {
            self.text_edit.contents = Self::format_contents(self.value);
        }

        drop(state);

        let layout = self.text_edit.layout(ctx, parent_id, available);
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

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        self.text_edit.min_size(ctx, available)
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
        // Set up internal callback so we can get the result from on_changed and
        // transform the value
        let (cb, tk) = ctx.create_internal_callback();
        self.text_edit.on_changed = Some(cb);

        // If we are dragging the slider, intercept and remove any mouse release
        // events to prevent the TextEdit from focusing.
        let patched: Vec<Event>;
        let child_events = {
            let state = ctx.memory.get::<DragValueState>(layout.widget_id);
            if state.dragging {
                patched = events
                    .iter()
                    .filter(|p| !matches!(p, Event::MouseReleased(MouseButton::Primary)))
                    .cloned()
                    .collect();
                patched.as_slice()
            } else {
                events
            }
        };

        let mut status = self
            .text_edit
            .on_event(ctx, layout, cursor_position, child_events);

        let mut state = ctx.memory.get_mut::<DragValueState>(layout.widget_id);

        // Check if the component just lost focus during this frame
        let focused_now = ctx.is_focused(layout.widget_id);
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
            // Clear dragging state when focus mode is active. Prevents some bugs
            state.dragging = false;

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
        } else {
            // Update drag state
            let is_mouse_over = layout.bounds.contains(cursor_position);
            if is_mouse_over
                && ctx.input_state.mouse_state.delta().length_sq() > 1.0
                && ctx
                    .input_state
                    .mouse_state
                    .button_state
                    .is_down(MouseButton::Primary)
            {
                state.dragging = true;
            }
            for event in events {
                match &event {
                    Event::MouseReleased(MouseButton::Primary) => {
                        state.dragging = false;
                    }
                    _ => (),
                }
            }

            if state.dragging {
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
        }

        status
    }

    // WIP: The behavior is different depending on the focus state:
    //
    // - When the widget is focused, this behaves like a regular TextEdit.
    // It will emit on_changed events when the thing inside the TextEdit can
    // be parsed as a float. This means we need to capture the internal
    // string returned by the TextEdit's on_change and store it in transient
    // memory. It's thus probably a bad idea to allocate the string inside
    // the child TextEdit during ::new(), because in some cases that will be
    // replaced. The string will be "patched" during `layout`, where we can
    // access the context, to be either the formatted float contents (when
    // not in edit mode), or the string stored in transient memory (when in
    // edit mode).
    //
    // - When the widget is not focused, the string contents are overwritten
    // by the provided float value. The on_event code for the TextEdit is
    // not even run. Instead, we handle drag events here.
    //
    // We currently have the first mode working, but the second mode
    // (non-focused widget) is not implemented yet.
}
