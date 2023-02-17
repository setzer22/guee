use epaint::{Pos2, Rect, Vec2};

use crate::widget_id::WidgetId;

pub struct Layout {
    // Bounds of this node. When creating this in a `layout` callback, it is
    // relative to its parent. The engine will convert the bounds to absolute
    // coordinates before feeding it to `draw`.
    pub bounds: Rect,
    /// The widget id. Uniquely identifies this widget in the state tree. Ids
    /// are sometimes used by event handling code to track volatile state from
    /// frame to frame.
    pub widget_id: WidgetId,
    // Children of this node.
    pub children: Vec<Layout>,
}

#[derive(Copy, Clone, Debug)]
pub struct LayoutHints {
    pub size_hints: SizeHints,
    pub weight: u32,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum Align {
    #[default]
    Start,
    End,
    Center,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum SizeHint {
    #[default]
    Shrink,
    Fill,
}

impl SizeHint {
    pub fn ignore_force_warning(struct_name: &str) {
        log::warn!(
            concat!(
                "{0} was requested to layout with force_shrink enabled. ",
                "It is an error to use {0} inside another flex container, ",
                "this request will be ignored."
            ),
            struct_name
        );
    }

    pub fn or_force(self, force_shrink: bool) -> Self {
        if force_shrink {
            Self::Shrink
        } else {
            self
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct SizeHints {
    pub width: SizeHint,
    pub height: SizeHint,
}

impl Default for LayoutHints {
    fn default() -> Self {
        Self {
            size_hints: Default::default(),
            weight: 1,
        }
    }
}

impl LayoutHints {
    pub fn shrink() -> Self {
        LayoutHints {
            size_hints: SizeHints {
                width: SizeHint::Shrink,
                height: SizeHint::Shrink,
            },
            weight: 0,
        }
    }

    pub fn fill() -> Self {
        LayoutHints {
            size_hints: SizeHints {
                width: SizeHint::Fill,
                height: SizeHint::Fill,
            },
            weight: 1,
        }
    }

    pub fn fill_vertical() -> Self {
        LayoutHints {
            size_hints: SizeHints {
                width: SizeHint::Shrink,
                height: SizeHint::Fill,
            },
            weight: 1,
        }
    }

    pub fn fill_horizontal() -> Self {
        LayoutHints {
            size_hints: SizeHints {
                width: SizeHint::Fill,
                height: SizeHint::Shrink,
            },
            weight: 1,
        }
    }
}

impl Layout {
    pub fn with_children(widget_id: WidgetId, size: Vec2, children: Vec<Layout>) -> Self {
        Self {
            bounds: Rect::from_min_size(Pos2::ZERO, size),
            children,
            widget_id,
        }
    }

    pub fn leaf(widget_id: WidgetId, size: Vec2) -> Self {
        Self {
            bounds: Rect::from_min_size(Pos2::ZERO, size),
            children: vec![],
            widget_id,
        }
    }

    pub fn translate(&mut self, translation: Vec2) {
        self.bounds = self.bounds.translate(translation);
    }

    pub fn translate_x(&mut self, dx: f32) {
        self.bounds = self.bounds.translate(Vec2::new(dx, 0.0));
    }

    pub fn translate_y(&mut self, dy: f32) {
        self.bounds = self.bounds.translate(Vec2::new(0.0, dy));
    }

    pub fn translate_cross(&mut self, axis: Axis, d: f32) {
        match axis {
            Axis::Vertical => self.translate_x(d),
            Axis::Horizontal => self.translate_y(d),
        };
    }

    pub fn translate_main(&mut self, axis: Axis, d: f32) {
        match axis {
            Axis::Vertical => self.translate_y(d),
            Axis::Horizontal => self.translate_x(d),
        };
    }

    pub fn translated(mut self, translation: Vec2) -> Self {
        self.translate(translation);
        self
    }

    pub fn clear_translation(self) -> Self {
        let delta = self.bounds.min.to_vec2();
        self.translated(-delta)
    }

    pub fn to_absolute(&mut self, parent_offset: Vec2) {
        self.bounds = self.bounds.translate(parent_offset);
        for ch in &mut self.children {
            ch.to_absolute(self.bounds.min.to_vec2())
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Axis {
    Vertical,
    Horizontal,
}

pub trait AxisDirections {
    type Output;
    fn main_dir(&self, axis: Axis) -> Self::Output;
    fn cross_dir(&self, axis: Axis) -> Self::Output;
}

impl AxisDirections for SizeHints {
    type Output = SizeHint;
    fn main_dir(&self, axis: Axis) -> SizeHint {
        match axis {
            Axis::Vertical => self.height,
            Axis::Horizontal => self.width,
        }
    }

    fn cross_dir(&self, axis: Axis) -> SizeHint {
        match axis {
            Axis::Vertical => self.width,
            Axis::Horizontal => self.height,
        }
    }
}

impl AxisDirections for Vec2 {
    type Output = f32;

    fn main_dir(&self, axis: Axis) -> Self::Output {
        match axis {
            Axis::Vertical => self.y,
            Axis::Horizontal => self.x,
        }
    }

    fn cross_dir(&self, axis: Axis) -> Self::Output {
        match axis {
            Axis::Vertical => self.x,
            Axis::Horizontal => self.y,
        }
    }
}

impl Axis {
    pub fn new_vec2(&self, main: f32, cross: f32) -> Vec2 {
        match self {
            Axis::Vertical => Vec2::new(cross, main),
            Axis::Horizontal => Vec2::new(main, cross),
        }
    }

    pub fn vec2_add_to_main(&self, v: Vec2, delta: f32) -> Vec2 {
        match self {
            Axis::Vertical => Vec2::new(v.x, v.y + delta),
            Axis::Horizontal => Vec2::new(v.x + delta, v.y),
        }
    }

    pub fn vec2_add(&self, v: Vec2, delta_main: f32, delta_cross: f32) -> Vec2 {
        match self {
            Axis::Vertical => Vec2::new(v.x + delta_cross, v.y + delta_main),
            Axis::Horizontal => Vec2::new(v.x + delta_main, v.y + delta_cross),
        }
    }

    pub fn vec2_scale(&self, v: Vec2, scale_main: f32, scale_cross: f32) -> Vec2 {
        match self {
            Axis::Vertical => Vec2::new(v.x * scale_cross, v.y * scale_main),
            Axis::Horizontal => Vec2::new(v.x * scale_main, v.y * scale_cross),
        }
    }
}
