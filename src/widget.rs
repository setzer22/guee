use epaint::Vec2;

use crate::{context::Context, layout::{Layout, LayoutHints, SizeHints}};

pub trait Widget: dyn_clone::DynClone {
    fn layout(&mut self, ctx: &Context, available: Vec2) -> Layout;
    fn draw(&mut self, ctx: &Context, layout: &Layout);
    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2;
    fn size_hints(&mut self) -> SizeHints;
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
