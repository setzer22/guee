use std::sync::Arc;

use base_widgets::{box_container::BoxContainer, button::Button, text::Text};
use context::Context;
use egui_wgpu::{winit::Painter, WgpuConfiguration};
use epaint::{
    text::FontDefinitions, textures::TexturesDelta, ClippedShape, Color32, FontId, Fonts, Galley,
    Pos2, Rect, Shape, Stroke, TessellationOptions, TextShape, TextureId, Vec2,
};
use input::EventStatus;
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

pub mod input;

pub mod base_widgets;

fn main() {
    let mut button_column = DynWidget::new(
        BoxContainer::horizontal()
            .contents(
                (0..5)
                    .map(|i| {
                        DynWidget::new(
                            Button::builder()
                                .contents(DynWidget::new(
                                    Text::builder()
                                        .contents(if i == 2 {
                                            "A\nA\nA\nA\nA\nA".into()
                                        } else {
                                            "AA".repeat(i + 1)
                                        })
                                        .build(),
                                ))
                                .padding(Vec2::new(15.0, 15.0))
                                .hints(LayoutHints {
                                    size_hints: SizeHints {
                                        width: if i == 3 {
                                            layout::SizeHint::Fill
                                        } else {
                                            layout::SizeHint::Shrink
                                        },
                                        height: if i == 4 {
                                            SizeHint::Fill
                                        } else {
                                            layout::SizeHint::Shrink
                                        },
                                    },
                                    weight: if i == 4 { 2 } else { 1 },
                                })
                                .build(),
                        )
                    })
                    .collect_vec(),
            )
            .main_align(Align::End)
            .cross_align(Align::Center)
            .build(),
    );

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
