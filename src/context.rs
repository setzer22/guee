use std::{any::Any, cell::RefCell};

use epaint::{text::FontDefinitions, Fonts, Shape, Vec2};

use crate::{callback::Callback, input::InputState, widget::DynWidget};

pub struct Context {
    pub fonts: Fonts,
    pub shapes: RefCell<Vec<Shape>>,
    pub input_state: InputState,
    pub callbacks: RefCell<Vec<Callback>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            fonts: Fonts::new(1.0, 1024, FontDefinitions::default()),
            shapes: Default::default(),
            input_state: Default::default(),
            callbacks: Default::default(),
        }
    }
    pub fn run(&mut self, widget: &mut DynWidget, state: &mut dyn Any) {
        let mut layout = widget.widget.layout(self, Vec2::new(800.0, 600.0));
        layout.to_absolute(Vec2::ZERO);
        let events = std::mem::take(&mut self.input_state.ev_buffer);
        for ev in events {
            widget
                .widget
                // Pass list of events to on_event
                .on_event(self, &layout, self.input_state.mouse_state.position, &ev);
        }
        widget.widget.draw(self, &layout);
        for callback in self.callbacks.borrow_mut().drain(..) {
            dbg!("callback");
            callback.call(state);
        }
    }

    pub fn push_callback(&self, c: Callback) {
        self.callbacks.borrow_mut().push(c)
    }
}
