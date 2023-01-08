use std::{borrow::Borrow, cell::RefCell, sync::Arc};

use context::Context;
use egui_wgpu::{winit::Painter, WgpuConfiguration};
use epaint::{
    text::FontDefinitions, textures::TexturesDelta, ClippedShape, Color32, FontId, Fonts, Galley,
    Pos2, Rect, RectShape, Rounding, Shape, Stroke, TessellationOptions, TextShape, TextureId,
    Vec2,
};
use itertools::Itertools;
use layout::{Align, Layout, LayoutHints, SizeHints};
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
        let mut contents_layout = self
            .contents
            .widget
            .layout(ctx, available - padding)
            .translated(padding);

        let size_hints = self.size_hints();
        let width = match size_hints.width {
            layout::SizeHint::Shrink => contents_layout.bounds.width() + 2.0 * padding.x,
            layout::SizeHint::Fill => available.x,
        };
        let height = match size_hints.height {
            layout::SizeHint::Shrink => contents_layout.bounds.height() + 2.0 * padding.y,
            layout::SizeHint::Fill => available.y,
        };

        contents_layout
            .translate_x((width - 2.0 * padding.x - contents_layout.bounds.width()) * 0.5);

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

    fn size_hints(&mut self) -> layout::SizeHints {
        self.hints.size_hints
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
        let galley = self.ensure_galley(&ctx.fonts, available.x);
        galley.rect.size()
    }

    fn size_hints(&mut self) -> layout::SizeHints {
        SizeHints {
            width: layout::SizeHint::Shrink,
            height: layout::SizeHint::Shrink,
        }
    }
}

#[derive(Clone)]
pub struct VBoxContainer {
    contents: Vec<DynWidget>,
    separation: f32,
    layout_hints: LayoutHints,
    main_align: Align,
    cross_align: Align,
}

impl Widget for VBoxContainer {
    fn layout(&mut self, ctx: &Context, available: Vec2) -> Layout {
        let cross_width = match self.layout_hints.size_hints.width {
            layout::SizeHint::Shrink => self.min_size(ctx, available).x,
            layout::SizeHint::Fill => available.x,
        };

        let mut main_offset = 0.0;
        let mut children = vec![];
        for ch in &mut self.contents {
            let available = Vec2::new(cross_width, available.y - main_offset);
            let ch_layout = ch
                .widget
                .layout(ctx, available)
                .clear_translation()
                .translated(Vec2::Y * main_offset);
            main_offset += ch_layout.bounds.height() + self.separation;
            children.push(ch_layout)
        }

        // Apply cross-axis alignment
        for (ch, ch_layout) in self.contents.iter_mut().zip(children.iter_mut()) {
            match ch.widget.size_hints().width {
                layout::SizeHint::Shrink => match self.cross_align {
                    Align::Start => {}
                    Align::End => {
                        ch_layout.translate_x(cross_width - ch_layout.bounds.width());
                    }
                    Align::Center => {
                        ch_layout.translate_x((cross_width - ch_layout.bounds.width()) * 0.5);
                    }
                },
                layout::SizeHint::Fill => {
                    // No alignment needed.
                }
            }
        }

        Layout::with_children(Vec2::new(cross_width, main_offset), children)

        // WIP: For the children that want to fill on the main (vertical) axis,
        // compute the remaining width and redistribution based on weight.
        //
        // WIP2: There's no such thing as a cross_align of Expand. Individual
        // elements are expanded or aligned based on their size hints. Having
        // both is redundant.
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        for (child, layout) in self.contents.iter_mut().zip(layout.children.iter()) {
            child.widget.draw(ctx, layout);
        }
    }

    fn min_size(&mut self, ctx: &Context, available: Vec2) -> Vec2 {
        let mut size_x = 0.0;
        let mut size_y = 0.0;

        for c in &mut self.contents {
            let c_available = Vec2::new(available.x, available.y - size_y);
            let s = c.widget.min_size(ctx, c_available);
            size_x = f32::max(size_x, s.x);
            size_y += s.y;
        }

        Vec2::new(size_x, size_y)
    }

    fn size_hints(&mut self) -> layout::SizeHints {
        self.layout_hints.size_hints
    }
}

fn main() {
    let mut button_column = DynWidget::new(VBoxContainer {
        contents: (0..10)
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
                            width: if i % 2 == 0 {
                                layout::SizeHint::Shrink
                            } else {
                                layout::SizeHint::Fill
                            },
                            ..Default::default()
                        },
                        ..Default::default()
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
        main_align: Align::Start,
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
