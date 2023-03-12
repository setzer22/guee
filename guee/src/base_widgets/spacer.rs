use epaint::{Pos2, Vec2};
use guee_derives::Builder;

use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints, SizeHint, SizeHints},
    widget::Widget,
    widget_id::WidgetId,
};

#[derive(Builder)]
#[builder(widget)]
pub struct Spacer {
    min_size: Vec2,
    layout_hints: LayoutHints,
}

impl Spacer {
    pub fn fill_h(weight: u32) -> Self {
        Self {
            min_size: Vec2::ZERO,
            layout_hints: LayoutHints {
                size_hints: SizeHints {
                    width: SizeHint::Fill,
                    height: SizeHint::Shrink,
                },
                weight,
            },
        }
    }

    pub fn fill_v(weight: u32) -> Self {
        Self {
            min_size: Vec2::ZERO,
            layout_hints: LayoutHints {
                size_hints: SizeHints {
                    width: SizeHint::Shrink,
                    height: SizeHint::Fill,
                },
                weight,
            },
        }
    }

    pub fn v(len: f32) -> Self {
        Self {
            min_size: Vec2::new(0.0, len),
            layout_hints: LayoutHints {
                size_hints: SizeHints {
                    width: SizeHint::Shrink,
                    height: SizeHint::Shrink,
                },
                weight: 1,
            },
        }
    }

    pub fn h(len: f32) -> Self {
        Self {
            min_size: Vec2::new(len, 0.0),
            layout_hints: LayoutHints {
                size_hints: SizeHints {
                    width: SizeHint::Shrink,
                    height: SizeHint::Shrink,
                },
                weight: 1,
            },
        }
    }
}

impl Widget for Spacer {
    fn layout(
        &mut self,
        _ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let widget_id = parent_id.with("spacer");
        let width = match self.layout_hints.size_hints.width.or_force(force_shrink) {
            SizeHint::Shrink => self.min_size.x,
            SizeHint::Fill => available.x,
        };
        let height = match self.layout_hints.size_hints.height.or_force(force_shrink) {
            SizeHint::Shrink => self.min_size.y,
            SizeHint::Fill => available.y,
        };
        Layout::leaf(widget_id, Vec2::new(width, height))
    }

    fn draw(&mut self, _ctx: &Context, _layout: &Layout) {
        // No need to draw
    }

    fn layout_hints(&self) -> LayoutHints {
        self.layout_hints
    }

    fn on_event(
        &mut self,
        _ctx: &Context,
        _layout: &Layout,
        _cursor_position: Pos2,
        _event: &[Event],
        _status: &mut EventStatus,
    ) {
    }
}
