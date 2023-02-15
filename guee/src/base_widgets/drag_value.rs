use epaint::{Pos2, Vec2};
use guee_derives::Builder;

use crate::prelude::*;

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
        let this: &dyn Widget = self;
        this.layout_hints()
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

        let status = self
            .text_edit
            .on_event(ctx, layout, cursor_position, events);

        let mut state = ctx.memory.get_mut::<DragValueState>(layout.widget_id);

        // Check if the component just lost focus during this frame
        let focused_now = ctx.is_focused(layout.widget_id);
        let did_blur = state.last_focus_state != focused_now && !focused_now;
        state.last_focus_state = focused_now;

        // When the TextEdit is focused, it should behave like a regular
        // TextEdit, letting the user write anything in the text box
        if focused_now {
            if let Some(result) = ctx.poll_callback_result(tk) {
                // If the inner text changed, replace the contents in transient state
                state.string_contents = result.clone();

                // Additionally, if the contents can be parsed as float, emit
                // our on_changed event
                if let Some(new_value) = Self::contents_from_string(&result) {
                    if let Some(on_changed) = self.on_changed.take() {
                        ctx.dispatch_callback(on_changed, new_value);
                    }
                }
            }
        } else {
            if did_blur {
                state.string_contents = Self::format_contents(self.value);
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
}
