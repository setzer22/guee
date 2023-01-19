use epaint::{Pos2, Vec2};

use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints},
    widget_id::WidgetId,
};

pub trait Widget {
    fn layout(&mut self, ctx: &Context, parent_id: WidgetId, available: Vec2) -> Layout;
    fn draw(&mut self, ctx: &Context, layout: &Layout);
    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2;
    fn layout_hints(&self) -> LayoutHints;
    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus;
}

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

impl<T> From<T> for DynWidget
where
    T: Widget + 'static,
{
    fn from(value: T) -> Self {
        DynWidget::new(value)
    }
}

pub trait ToDynWidget {
    fn to_dyn(self) -> DynWidget;
}

impl<T> ToDynWidget for T
where
    T: Widget + 'static,
{
    fn to_dyn(self) -> DynWidget {
        DynWidget::new(self)
    }
}
