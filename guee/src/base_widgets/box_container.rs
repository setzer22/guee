use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Align, Axis, AxisDirections, Layout, LayoutHints, SizeHint},
    widget::{DynWidget, Widget},
    widget_id::{IdGen, WidgetId},
};
use epaint::{Pos2, Vec2};
use guee_derives::Builder;
use itertools::Itertools;

#[derive(Builder)]
#[builder(widget)]
pub struct BoxContainer {
    id: IdGen,
    axis: Axis,
    contents: Vec<DynWidget>,
    #[builder(default = 3.0)]
    separation: f32,
    #[builder(default)]
    layout_hints: LayoutHints,
    #[builder(default)]
    main_align: Align,
    #[builder(default)]
    cross_align: Align,
}

impl BoxContainer {
    pub fn vertical(id_gen: IdGen, contents: Vec<DynWidget>) -> BoxContainer {
        Self::new(id_gen, Axis::Vertical, contents)
    }

    pub fn horizontal(id_gen: IdGen, contents: Vec<DynWidget>) -> BoxContainer {
        Self::new(id_gen, Axis::Horizontal, contents)
    }
}

impl Widget for BoxContainer {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let widget_id = self.id.resolve(parent_id);

        // We do this, so the rest of the code can assume child list is non-empty
        if self.contents.is_empty() {
            return Layout::leaf(widget_id, Vec2::ZERO);
        }

        // Compute the child layouts as if they were all in shrink mode. This
        // helps compute some metrics later on.
        let shrink_child_layouts = self
            .contents
            .iter_mut()
            .map(|x| x.widget.layout(ctx, parent_id, available, true))
            .collect_vec();

        // The `cross_space` is the amount of space this box container will
        // occupy in the cross axis direction.
        let axis = self.axis;
        let cross_space = match self
            .layout_hints
            .size_hints
            .cross_dir(axis)
            .or_force(force_shrink)
        {
            SizeHint::Shrink => {
                let axis = self.axis;
                let mut size_main = 0.0;
                let mut size_cross = 0.0;

                for c_layout in &shrink_child_layouts {
                    let c_available = axis.vec2_add_to_main(available, -size_main);
                    let s = c_layout.bounds.size();

                    size_cross = f32::max(size_cross, s.cross_dir(axis));
                    size_main += s.main_dir(axis);
                }
                size_cross
            }
            SizeHint::Fill => available.cross_dir(axis),
        };

        // Some early computations
        let mut total_filled_weight = 0;
        let mut total_shrink_space = 0.0;
        let mut fill_child_count = 0;
        for (c, shrk) in self.contents.iter_mut().zip(&shrink_child_layouts) {
            match c
                .widget
                .layout_hints()
                .size_hints
                .main_dir(axis)
                .or_force(force_shrink)
            {
                SizeHint::Shrink => {
                    total_shrink_space += shrk.bounds.size().main_dir(axis);
                }
                SizeHint::Fill => {
                    fill_child_count += 1;
                    total_filled_weight += c.widget.layout_hints().weight;
                }
            }
        }
        let total_separation = self.separation * (self.contents.len() - 1) as f32;
        // How much total space elements on the main axis would get to grow
        let wiggle_room = available.main_dir(axis) - (total_shrink_space + total_separation);

        let mut main_offset = 0.0;
        let mut children = vec![];
        for ch in &mut self.contents {
            let c_available = match ch
                .widget
                .layout_hints()
                .size_hints
                .main_dir(axis)
                .or_force(force_shrink)
            {
                SizeHint::Shrink => {
                    axis.new_vec2(available.main_dir(axis) - main_offset, cross_space)
                }
                SizeHint::Fill => axis.new_vec2(
                    wiggle_room
                        * (ch.widget.layout_hints().weight as f32 / total_filled_weight as f32),
                    cross_space,
                ),
            };

            let axis_vec = match axis {
                Axis::Vertical => Vec2::Y,
                Axis::Horizontal => Vec2::X,
            };
            let ch_layout = ch
                .widget
                .layout(ctx, widget_id, c_available, force_shrink)
                .clear_translation()
                .translated(axis_vec * main_offset);
            main_offset += ch_layout.bounds.size().main_dir(axis) + self.separation;
            children.push(ch_layout)
        }

        // Apply cross-axis alignment
        for (ch, ch_layout) in self.contents.iter().zip(children.iter_mut()) {
            match ch
                .widget
                .layout_hints()
                .size_hints
                .cross_dir(axis)
                .or_force(force_shrink)
            {
                SizeHint::Shrink => match self.cross_align {
                    Align::Start => {}
                    Align::End => {
                        ch_layout.translate_cross(
                            axis,
                            cross_space - ch_layout.bounds.size().cross_dir(axis),
                        );
                    }
                    Align::Center => {
                        ch_layout.translate_cross(
                            axis,
                            (cross_space - ch_layout.bounds.size().cross_dir(axis)) * 0.5,
                        );
                    }
                },
                SizeHint::Fill => {
                    // No alignment needed.
                }
            }
        }

        let content_main_size = main_offset;

        // Apply main axis alignment
        if fill_child_count == 0 {
            // Only when there's no child set to fill on the main axis, we have
            // to do alignment because otherwise this layout takes full space
            let offset = match self.main_align {
                Align::Start => 0.0,
                Align::End => available.main_dir(axis) - content_main_size,
                Align::Center => (available.main_dir(axis) - content_main_size) * 0.5,
            };

            for ch_layout in &mut children {
                ch_layout.translate_main(axis, offset);
            }
        }

        Layout::with_children(
            widget_id,
            axis.new_vec2(
                children
                    .last()
                    // The rightmost or bottommost position, depending on axis
                    .map(|x| x.bounds.max.to_vec2().main_dir(axis))
                    .unwrap_or(0.0),
                cross_space,
            ),
            children,
        )
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        for (child, layout) in self.contents.iter_mut().zip(layout.children.iter()) {
            child.widget.draw(ctx, layout);
        }
    }

    fn layout_hints(&self) -> LayoutHints {
        self.layout_hints
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        for (ch, ch_layout) in self.contents.iter_mut().zip(layout.children.iter()) {
            if ch.widget.on_event(ctx, ch_layout, cursor_position, events) == EventStatus::Consumed
            {
                return EventStatus::Consumed;
            }
        }
        EventStatus::Ignored
    }
}
