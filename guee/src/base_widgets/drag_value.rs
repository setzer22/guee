use std::ops::RangeInclusive;

use epaint::{emath::Align2, Pos2, RectShape, Rounding, Vec2};
use guee_derives::Builder;

use crate::{extension_traits::Vec2Ext, input::MouseButton, prelude::*};

#[derive(Builder)]
#[builder(widget, rename_new = "__new")]
pub struct DragValue {
    /// The underlying float value that this drag value is "editing".
    pub value: f64,

    /// The base speed. After each discrete increment of mouse drag movement,
    /// how much the underlying value is going to increase / decrease.
    #[builder(default = 0.1)]
    pub speed: f64,

    /// When set, shows a scale selector allowing the user to adjust the base
    /// speed by a multiplier.
    #[builder(default)]
    pub scale_selector: Option<ScaleSelector>,

    /// When set, the scale selector will be first initialized at this index
    /// instead of the half point.
    #[builder(default)]
    pub default_scale_selector_index: Option<usize>,

    /// A recommended range of values for this slider. The values returned can
    /// go beyond the limits when using the text edit feature, or when dragging
    /// again after the slider reached the soft max/min value
    #[builder(default = -f64::INFINITY..=f64::INFINITY)]
    pub soft_range: RangeInclusive<f64>,

    /// The range of movement for this slider. The values returned can never go
    /// above or beyond those limits.
    #[builder(default = -f64::INFINITY..=f64::INFINITY)]
    pub hard_range: RangeInclusive<f64>,

    /// The inner value will be rounded to this number of decimal values. If set
    /// to 0, this acts as an Integer DragValue
    #[builder(default = 4)]
    pub num_decimals: u32,

    /// Emitted when the value has changed.
    #[builder(strip_option)]
    pub on_changed: Option<Callback<f64>>,

    /// Inner TextEdit, used to implement some functionalities for this widget
    /// avoiding code repetition.
    #[builder(skip, default = TextEdit::new(IdGen::key(""), "".to_string()))]
    pub text_edit: TextEdit,
}

#[derive(Clone, Debug)]
pub struct ScaleSelector {
    /// True for left, false for right
    pub show_left_of_widget: bool,
    /// List of speed multiplers
    pub speeds: Vec<f64>,
    /// List of labels for the speed multiplers
    pub labels: Vec<String>,
}

impl ScaleSelector {
    pub fn float_7vals() -> Self {
        Self {
            show_left_of_widget: false,
            speeds: vec![100.0, 10.0, 1.0, 0.1, 0.01, 0.001, 0.0001],
            labels: ["100", "10", "1", ".1", ".01", ".001", ".0001"]
                .map(|x| x.to_string())
                .to_vec(),
        }
    }

    pub fn int_3vals() -> Self {
        Self {
            show_left_of_widget: false,
            speeds: vec![100.0, 10.0, 1.0],
            labels: ["100", "10", "1"].map(|x| x.to_string()).to_vec(),
        }
    }
}

impl ScaleSelector {
    pub fn new(speeds: Vec<f64>, labels: Vec<String>, left: bool) -> Self {
        assert_eq!(
            speeds.len(),
            labels.len(),
            "Should provide the same amount of speeds and labels"
        );
        assert!(
            speeds.len() > 1,
            "The scale selector expects at least two different speeds to choose from."
        );
        Self {
            show_left_of_widget: left,
            speeds,
            labels,
        }
    }

    fn len(&self) -> usize {
        self.speeds.len()
    }
}

pub struct DragValueState {
    /// The focus state for the widget during the last frame.
    pub last_focus_state: bool,

    /// The dragging state for the widget during the last frame.
    pub last_drag_state: bool,

    /// The string contents of the inner TextEdit. Stored here because we can't
    /// rely on the app state storing it.
    pub string_contents: String,

    /// Accumulated amount of mouse delta for the current drag event.
    pub acc_drag: Vec2,

    /// Should the scale selector be drawn? When true, `selected_row` should
    /// always be set.
    pub draw_scale_selector: bool,

    /// The currently selected row for the scale selector.
    pub selected_row: Option<usize>,

    /// True when the current drag event started at the upper soft limit. This
    /// allows the slider to go past the soft max.
    pub upper_soft_limit: bool,

    /// True when the current drag event started at the bottom soft limit. This
    /// allows the slider to go past the soft min.
    pub lower_soft_limit: bool,
}

impl DragValue {
    pub fn format_contents(contents: f64, num_decimals: usize) -> String {
        format!("{contents:.num_decimals$}")
    }

    pub fn contents_from_string(s: &str) -> Option<f64> {
        s.parse().ok()
    }

    pub fn new(id: IdGen, value: f64) -> Self {
        DragValue {
            value,
            text_edit: TextEdit::new(
                id,
                // The string contents are patched later, during `Layout`,
                // depending on this widget's internal state.
                String::new(),
            ),
            ..Self::__new(value)
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

    fn clamp_and_round_value(&self, state: &DragValueState, val: f64) -> f64 {
        let lower_bound = if state.lower_soft_limit {
            *self.hard_range.start()
        } else {
            *self.soft_range.start()
        };
        let upper_bound = if state.upper_soft_limit {
            *self.hard_range.end()
        } else {
            *self.soft_range.end()
        };

        // Clamp base value
        let val = val.clamp(lower_bound, upper_bound);

        // Round to decimal places
        let pow = 10.0f64.powi(self.num_decimals as i32);
        (val * pow).round() / pow
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
                last_drag_state: false,
                string_contents: Self::format_contents(self.value, self.num_decimals as usize),
                acc_drag: Vec2::ZERO,
                selected_row: None,
                draw_scale_selector: false,
                upper_soft_limit: false,
                lower_soft_limit: false,
            },
        );

        if is_focused {
            self.text_edit.contents = state.string_contents.clone();
        } else {
            self.text_edit.contents = Self::format_contents(self.value, self.num_decimals as usize);
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
        self.text_edit.draw(ctx, layout);
        let state = ctx.memory.get::<DragValueState>(layout.widget_id);

        if state.draw_scale_selector {
            let scale_selector = self
                .scale_selector
                .as_ref()
                .expect("The draw_scale_selector property was set but no scale selector exists");
            let selected_row = state.selected_row.expect("Should be initialized");

            let padding = Vec2::new(4.0, 1.0);
            let size = Vec2::new(60.0, 27.0);

            let top_left = if scale_selector.show_left_of_widget {
                layout.bounds.left_center()
                    - Vec2::new(padding.x + size.x, size.y * (0.5 + selected_row as f32))
            } else {
                layout.bounds.right_center()
                    - Vec2::new(-padding.x, size.y * (0.5 + selected_row as f32))
            };

            let mut painter = ctx.painter();

            painter.with_overlay(|painter| {
                for (i, label) in scale_selector.labels.iter().enumerate() {
                    let pos = top_left + Vec2::new(0.0, size.y) * i as f32;

                    painter.rect(RectShape {
                        rect: Rect::from_min_size(pos, size),
                        rounding: Rounding::none(),
                        // TODO: THEME
                        fill: if selected_row == i {
                            color!("#373737B0")
                        } else {
                            color!("#212121B0")
                        },
                        stroke: Stroke::new(1.0, color!("#3c3c3c")),
                    });

                    painter.text(
                        pos + Vec2::new(size.x * 0.5, padding.y),
                        Align2::CENTER_TOP,
                        label,
                        // TODO: THEME
                        FontId::proportional(14.0),
                    );
                }
            })
        }
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
        let dragging = ctx.claim_drag_event(layout.widget_id, layout.bounds, MouseButton::Primary);

        // A TextEdit normally focuses itself, but we are inhibiting that below
        // by not feeding it events unless it's focused.
        if layout.bounds.contains(cursor_position)
            && ctx
                .input_state
                .mouse
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

        // Check if the component was just focused or dragged during this frame
        let just_focused = state.last_focus_state != focused_now && focused_now;
        state.last_focus_state = focused_now;
        let just_dragged = dragging != state.last_drag_state && dragging;
        state.last_drag_state = dragging;

        if just_focused {
            // When first focused, the string contents are overriden with
            // whatever float value we have, so that when the editor gains focus
            // the string is like the user was seeing it in the UI. Displaying
            // the old value can lead to confusing results.
            state.string_contents = Self::format_contents(self.value, self.num_decimals as usize);
        }

        state.draw_scale_selector = dragging && self.scale_selector.is_some();

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
                        ctx.dispatch_callback(
                            on_changed,
                            self.clamp_and_round_value(&state, new_value),
                        );
                    }
                }
            }
        } else if dragging {
            // Scale selector
            if let Some(scale_selector) = &self.scale_selector {
                // Check if a drag event started exactly this frame, and initialize
                // scale selector data.
                if just_dragged {
                    // NOTE: Only set the range if this is our first time editing this
                    // DragValue. Doing this remembers previous scale value from the
                    // last time the user touched this slider, which provides better UX:
                    // The range they picked was probably a good one.
                    if state.selected_row.is_none() {
                        state.selected_row = Some(
                            self.default_scale_selector_index
                                .unwrap_or(scale_selector.len() / 2),
                        )
                    }
                    state.lower_soft_limit = self.value <= *self.soft_range.start();
                    state.upper_soft_limit = self.value >= *self.soft_range.end();
                }

                // Make sure the selected row always stays within bounds. This
                // could change if a different amount of range divisions is set
                // for this frame but old data was stored in memory.
                state.selected_row = state
                    .selected_row
                    .map(|s| s.clamp(0, scale_selector.len() - 1));
            }

            // Handle mouse movement
            const MOUSE_PRECISION: Vec2 = Vec2::new(20.0, 50.0);

            let modify_scale: bool = ctx.input_state.modifiers.ctrl_or_command;

            if modify_scale {
                // TODO: Do we need to handle scale in the delta?
                state.acc_drag += ctx.input_state.mouse.delta().y * Vec2::Y;
            } else {
                state.acc_drag += ctx.input_state.mouse.delta().x * Vec2::X;
            }

            let discrete_increments = (state.acc_drag / MOUSE_PRECISION).floor();
            state.acc_drag = state.acc_drag.rem_euclid(MOUSE_PRECISION);

            let speed = match &self.scale_selector {
                Some(scale_selector) => {
                    let selected_row = state.selected_row.as_mut().expect("Should be initialized");
                    *selected_row = (*selected_row as isize - discrete_increments.y as isize)
                        .clamp(0, scale_selector.len() as isize - 1)
                        as usize;
                    self.speed * scale_selector.speeds[*selected_row]
                }
                None => self.speed,
            };

            let delta_value = discrete_increments.x as f64 * speed;
            let new_value = self.clamp_and_round_value(&state, self.value + delta_value);

            if let Some(on_changed) = self.on_changed.take() {
                ctx.dispatch_callback(on_changed, new_value);
                status = EventStatus::Consumed
            }
        }

        status
    }
}
