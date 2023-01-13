use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Align, Axis, AxisDirections, Layout, LayoutHints, SizeHint},
    widget::{DynWidget, Widget},
};
use epaint::{Pos2, Vec2};
use typed_builder::TypedBuilder;

#[derive(TypedBuilder)]
pub struct BoxContainer {
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
    pub fn vertical() -> BoxContainerBuilder<((Axis,), (), (), (), (), ())> {
        Self::builder().axis(Axis::Vertical)
    }

    pub fn horizontal() -> BoxContainerBuilder<((Axis,), (), (), (), (), ())> {
        Self::builder().axis(Axis::Horizontal)
    }
}

impl Widget for BoxContainer {
    fn layout(&mut self, ctx: &Context, available: Vec2) -> Layout {
        // We do this, so the rest of the code can assume child list is non-empty
        if self.contents.is_empty() {
            return Layout::leaf(Vec2::ZERO)
        }

        let axis = self.axis;
        let cross_space = match self.layout_hints.size_hints.cross_dir(axis) {
            SizeHint::Shrink => self.min_size(ctx, available).cross_dir(axis),
            SizeHint::Fill => available.cross_dir(axis),
        };

        // Some early computations
        let mut total_filled_weight = 0;
        let mut total_shrink_space = 0.0;
        let mut fill_child_count = 0;
        for c in &mut self.contents {
            match c.widget.layout_hints().size_hints.main_dir(axis) {
                SizeHint::Shrink => {
                    // TODO: This available here is not correct, some things
                    // like text wrapping may fail to compute.
                    total_shrink_space += c.widget.min_size(ctx, available).main_dir(axis);
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
            let c_available = match ch.widget.layout_hints().size_hints.main_dir(axis) {
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
                .layout(ctx, c_available)
                .clear_translation()
                .translated(axis_vec * main_offset);
            main_offset += ch_layout.bounds.size().main_dir(axis) + self.separation;
            children.push(ch_layout)
        }

        // Apply cross-axis alignment
        for (ch, ch_layout) in self.contents.iter().zip(children.iter_mut()) {
            match ch.widget.layout_hints().size_hints.cross_dir(axis) {
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
            Vec2::new(
                cross_space,
                children
                    .last()
                    // The rightmost or bottommost position, depending on axis
                    .map(|x| x.bounds.max.to_vec2().main_dir(axis))
                    .unwrap_or(0.0),
            ),
            children,
        )
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        for (child, layout) in self.contents.iter_mut().zip(layout.children.iter()) {
            child.widget.draw(ctx, layout);
        }
    }

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        let axis = self.axis;
        let mut size_main = 0.0;
        let mut size_cross = 0.0;

        for c in &mut self.contents {
            //Vec2::new(available.x, available.y - size_y);
            let c_available = axis.vec2_add_to_main(available, -size_main);
            let s = c.widget.min_size(ctx, c_available);

            size_cross = f32::max(size_cross, s.cross_dir(axis));
            size_main += s.main_dir(axis);
        }

        match axis {
            Axis::Vertical => Vec2::new(size_cross, size_main),
            Axis::Horizontal => Vec2::new(size_main, size_cross),
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
        event: &Event,
    ) -> EventStatus {
        for (ch, ch_layout) in self.contents.iter_mut().zip(layout.children.iter()) {
            if ch.widget.on_event(ctx, ch_layout, cursor_position, event) == EventStatus::Consumed {
                return EventStatus::Consumed;
            }
        }
        EventStatus::Ignored
    }
}
