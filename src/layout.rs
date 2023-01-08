use epaint::{Pos2, Rect, Vec2};

pub struct Layout {
    // Bounds of this node. When creating this in a `layout` callback, it is
    // relative to its parent. The engine will convert the bounds to absolute
    // coordinates before feeding it to `draw`.
    pub bounds: Rect,
    // Children of this node.
    pub children: Vec<Layout>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct LayoutHints {
    pub size_hints: SizeHints,
    pub min_size: Vec2,
    pub max_size: Vec2,
    pub weight: u32,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum Align {
    #[default]
    Start,
    End,
    Center,
}

#[derive(Copy, Clone, Debug, Default)]
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
