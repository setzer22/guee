use std::{borrow::Borrow, cell::RefCell, sync::Arc};

use context::Context;
use egui_wgpu::{winit::Painter, WgpuConfiguration};
use epaint::{
    text::FontDefinitions, textures::TexturesDelta, ClippedShape, Color32, FontId, Fonts, Galley,
    Pos2, Rect, RectShape, Rounding, Shape, Stroke, TessellationOptions, TextShape, TextureId,
    Vec2,
};
use itertools::Itertools;
use layout::{Align, Axis, AxisDirections, Layout, LayoutHints, SizeHint, SizeHints};
use widget::{DynWidget, Widget};
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

//pub mod epaint_shape_routine;
pub mod epaint_routine;

pub mod layout;

pub mod widget;

pub mod context;

#[derive(Clone)]
pub struct Button {
    pressed: bool,
    contents: DynWidget,
    hints: LayoutHints,
    padding: Vec2,
}

impl Widget for Button {
    fn layout(&mut self, ctx: &Context, available: Vec2) -> Layout {
        let padding = self.padding;
        let mut contents_layout = self.contents.widget.layout(ctx, available - padding);

        let size_hints = self.hints.size_hints;
        let width = match size_hints.width {
            layout::SizeHint::Shrink => contents_layout.bounds.width() + 2.0 * padding.x,
            layout::SizeHint::Fill => available.x,
        };
        let height = match size_hints.height {
            layout::SizeHint::Shrink => contents_layout.bounds.height() + 2.0 * padding.y,
            layout::SizeHint::Fill => available.y,
        };

        contents_layout.translate(Vec2::new(
            (width - contents_layout.bounds.width()) * 0.5,
            (height - contents_layout.bounds.height()) * 0.5,
        ));

        Layout::with_children(Vec2::new(width, height), vec![contents_layout])
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        ctx.shapes.borrow_mut().push(Shape::Rect(RectShape {
            rect: layout.bounds,
            rounding: Rounding::same(2.0),
            fill: Color32::from_rgba_unmultiplied(40, 200, 40, 50),
            stroke: Stroke::NONE,
        }));
        self.contents.widget.draw(ctx, &layout.children[0]);
    }

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        self.contents
            .widget
            .min_size(ctx, available - self.padding * 2.0)
            + self.padding * 2.0
    }

    fn layout_hints(&self) -> layout::LayoutHints {
        self.hints
    }
}

#[derive(Clone)]
pub struct Text {
    contents: String,
    last_galley: Option<Arc<Galley>>,
}

impl Text {
    pub fn ensure_galley(&mut self, fonts: &Fonts, wrap_width: f32) -> Arc<Galley> {
        let galley = fonts.layout(
            self.contents.clone(),
            FontId::proportional(14.0),
            Color32::BLACK,
            wrap_width,
        );
        self.last_galley = Some(galley.clone());
        galley
    }
}

impl Widget for Text {
    fn layout(&mut self, ctx: &Context, available: Vec2) -> Layout {
        Layout::leaf(self.min_size(ctx, available))
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let galley = self
            .last_galley
            .clone()
            .expect("Layout should be called before draw");
        ctx.shapes.borrow_mut().push(Shape::Text(TextShape {
            pos: layout.bounds.left_top(),
            galley,
            underline: Stroke::NONE,
            override_text_color: None,
            angle: 0.0,
        }));
    }

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        dbg!(available);
        let galley = self.ensure_galley(&ctx.fonts, available.x);
        galley.rect.size()
    }

    fn layout_hints(&self) -> layout::LayoutHints {
        LayoutHints {
            size_hints: SizeHints {
                width: SizeHint::Shrink,
                height: SizeHint::Shrink,
            },
            weight: 1,
        }
    }
}

#[derive(Clone)]
pub struct VBoxContainer {
    axis: Axis,
    contents: Vec<DynWidget>,
    separation: f32,
    layout_hints: LayoutHints,
    main_align: Align,
    cross_align: Align,
}

impl Widget for VBoxContainer {
    fn layout(&mut self, ctx: &Context, available: Vec2) -> Layout {
        println!("Start");
        println!("Cross space");
        let axis = self.axis;
        let cross_space = match self.layout_hints.size_hints.cross_dir(axis) {
            layout::SizeHint::Shrink => self.min_size(ctx, available).cross_dir(axis),
            layout::SizeHint::Fill => available.cross_dir(axis),
        };

        println!("Early computations");
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

        println!("Child layout");
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
                layout::SizeHint::Shrink => match self.cross_align {
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
                layout::SizeHint::Fill => {
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
            let s = c.widget.min_size(ctx, dbg!(c_available));

            size_cross = f32::max(size_cross, s.cross_dir(axis));
            size_main += s.main_dir(axis);
        }

        match axis {
            Axis::Vertical => Vec2::new(size_cross, size_main),
            Axis::Horizontal => dbg!(Vec2::new(size_main, size_cross)),
        }
    }

    fn layout_hints(&self) -> LayoutHints {
        self.layout_hints
    }
}

fn main() {
    let mut button_column = DynWidget::new(VBoxContainer {
        axis: Axis::Horizontal,
        contents: (0..5)
            .map(|i| {
                DynWidget::new(Button {
                    pressed: false,
                    contents: DynWidget::new(Text {
                        contents: "AA".repeat(i + 1),
                        last_galley: None,
                    }),
                    padding: Vec2::new(15.0, 15.0),
                    hints: LayoutHints {
                        size_hints: SizeHints {
                            width: /*if i % 2 == 0 {
                                layout::SizeHint::Shrink
                            } else {
                                layout::SizeHint::Fill
                            }*/ layout::SizeHint::Shrink,
                            height: /*if i == 4 || i == 6 {
                                layout::SizeHint::Fill
                            } else {
                                layout::SizeHint::Shrink
                            }*/ layout::SizeHint::Shrink,
                        },
                        weight: if i == 4 { 2 } else { 1 },
                    },
                })
            })
            .collect_vec(),
        separation: 3.0,
        layout_hints: LayoutHints {
            size_hints: SizeHints {
                width: layout::SizeHint::Shrink,
                ..Default::default()
            },
            ..Default::default()
        },
        main_align: Align::End,
        cross_align: Align::Center,
    });

    let ctx = Context {
        fonts: Fonts::new(1.0, 1024, FontDefinitions::default()),
        shapes: Default::default(),
    };

    let screen_size = Vec2::new(800.0, 600.0);
    let screen_rect = Rect::from_min_size(Pos2::new(0.0, 0.0), screen_size);

    ctx.run(&mut button_column);

    let clipped_primitives = epaint::tessellate_shapes(
        1.0,
        TessellationOptions::default(),
        ctx.fonts.font_image_size(),
        vec![],
        ctx.shapes
            .borrow_mut()
            .drain(..)
            .map(|x| ClippedShape(screen_rect, x))
            .collect_vec(),
    );

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Test GUI")
        .build(&event_loop)
        .unwrap();

    let mut painter = Painter::new(WgpuConfiguration::default(), 1, 0);
    unsafe { pollster::block_on(painter.set_window(Some(&window))).unwrap() };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            winit::event::Event::MainEventsCleared => {
                let mut textures_delta = TexturesDelta::default();
                if let Some(img_delta) = ctx.fonts.font_image_delta() {
                    textures_delta.set.push((TextureId::default(), img_delta));
                }
                painter.paint_and_update_textures(
                    1.0,
                    epaint::Rgba::from_rgb(0.7, 0.3, 0.3),
                    &clipped_primitives,
                    &textures_delta,
                );
            }
            winit::event::Event::WindowEvent { window_id, event } if window_id == window.id() => {
                match &event {
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    })
}
