use std::any::type_name;

use epaint::{Pos2, Rect, Vec2};
use guee_derives::Builder;

use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints},
    prelude::SizeHint,
    widget::{DynWidget, Widget},
    widget_id::{IdGen, WidgetId},
};

#[derive(Builder)]
#[builder(widget)]
pub struct StackContainer {
    id: IdGen,
    contents: Vec<(Vec2, DynWidget)>,
}

impl Widget for StackContainer {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool, // ignored, always expanded
    ) -> Layout {
        if force_shrink {
            SizeHint::ignore_force_warning(type_name::<Self>());
        }

        let widget_id = self.id.resolve(parent_id);

        let mut children_layouts = Vec::new();
        let mut current_rect = Rect::from_min_max(Pos2::ZERO, Pos2::ZERO);

        for (ch_offs, ch) in &mut self.contents {
            let available = available - *ch_offs;
            let ch_layout = ch
                .widget
                .layout(ctx, widget_id, available, false)
                .translated(*ch_offs);
            current_rect = current_rect.union(ch_layout.bounds);
            children_layouts.push(ch_layout);
        }

        Layout::with_children(widget_id, current_rect.size(), children_layouts)
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        for ((_, ch), ch_layout) in self.contents.iter_mut().zip(layout.children.iter()) {
            ch.widget.draw(ctx, ch_layout);
        }
    }

    fn layout_hints(&self) -> LayoutHints {
        LayoutHints::fill()
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        for ((_, ch), ch_layout) in self.contents.iter_mut().zip(&layout.children) {
            if let EventStatus::Consumed =
                ch.widget.on_event(ctx, ch_layout, cursor_position, events)
            {
                return EventStatus::Consumed;
            }
        }
        EventStatus::Ignored
    }
}
