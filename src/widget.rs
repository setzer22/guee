use epaint::{Pos2, Vec2};

use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints},
};

pub trait Widget: dyn_clone::DynClone {
    fn layout(&mut self, ctx: &Context, available: Vec2) -> Layout;
    fn draw(&mut self, ctx: &Context, layout: &Layout);
    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2;
    fn layout_hints(&self) -> LayoutHints;
    fn on_event(&mut self, layout: &Layout, cursor_position: Pos2, event: &Event) -> EventStatus;
}

dyn_clone::clone_trait_object!(Widget);

#[derive(Clone)]
pub struct DynWidget {
    pub widget: Box<dyn Widget>,
}

impl DynWidget {
    pub fn new<T: Widget + 'static>(w: T) -> Self {
        Self {
            widget: Box::new(w),
        }
    }
}
