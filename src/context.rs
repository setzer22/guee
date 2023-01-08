use std::cell::RefCell;

use epaint::{Fonts, Shape, Vec2};

use crate::widget::DynWidget;

pub struct Context {
    pub fonts: Fonts,
    pub shapes: RefCell<Vec<Shape>>,
}

impl Context {
    pub fn run(&self, widget: &mut DynWidget) {
        let mut layout = widget.widget.layout(self, Vec2::new(800.0, 600.0));
        layout.to_absolute(Vec2::ZERO);
        widget.widget.draw(self, &layout);
    }
}
