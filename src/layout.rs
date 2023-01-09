use epaint::{Pos2, Rect, Vec2};

pub struct Layout {
    // Bounds of this node. When creating this in a `layout` callback, it is
    // relative to its parent. The engine will convert the bounds to absolute
    // coordinates before feeding it to `draw`.
    pub bounds: Rect,
    // Children of this node.
    pub children: Vec<Layout>,
}

#[derive(Copy, Clone, Debug)]
pub struct LayoutHints {
    pub size_hints: SizeHints,
    pub weight: u32,
}

impl Default for LayoutHints {
    fn default() -> Self {
        Self {
            size_hints: Default::default(),
            weight: 1,
        }
    }
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

#[derive(Copy, Clone, Debug, Default)]
pub struct SizeHints {
    pub width: SizeHint,
    pub height: SizeHint,
}

impl Layout {
    pub fn with_children(size: Vec2, children: Vec<Layout>) -> Self {
        Self {
            bounds: Rect::from_min_size(Pos2::ZERO, size),
            children,
        }
    }

    pub fn leaf(size: Vec2) -> Self {
        Self {
            bounds: Rect::from_min_size(Pos2::ZERO, size),
            children: vec![],
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
}
