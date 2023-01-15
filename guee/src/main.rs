use std::sync::Arc;

use base_widgets::{
    box_container::BoxContainer, button::Button, margin_container::MarginContainer, spacer::Spacer,
    text::Text, text_edit::TextEdit,
};
use callback::Callback;
use context::Context;
use egui_wgpu::{winit::Painter, WgpuConfiguration};
use epaint::{
    text::FontDefinitions, textures::TexturesDelta, ClippedShape, Color32, FontId, Fonts, Galley,
    Pos2, Rect, Shape, Stroke, TessellationOptions, TextShape, TextureId, Vec2,
};
use input::{EventStatus, InputState};
use itertools::Itertools;
use layout::{Align, Axis, AxisDirections, Layout, LayoutHints, SizeHint, SizeHints};
use widget::{DynWidget, ToDynWidget, Widget};
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::widget_id::IdGen;

extern crate self as guee;

//pub mod epaint_shape_routine;
pub mod epaint_routine;

pub mod widget_id;

pub mod layout;

pub mod widget;

pub mod context;

pub mod input;

pub mod base_widgets;

pub mod callback;

#[derive(Default)]
pub struct AppState {
    items: Vec<String>,
}

fn view(state: &AppState) -> DynWidget {
    MarginContainer::new(
        IdGen::key("margin"),
        BoxContainer::vertical(
            IdGen::key("vbox"),
            vec![
                BoxContainer::vertical(
                    IdGen::key("items"),
                    state
                        .items
                        .iter()
                        .map(|it| Text::new(it.clone()).build())
                        .collect_vec(),
                )
                .layout_hints(LayoutHints::fill_horizontal())
                .cross_align(Align::Center)
                .build(),
                Spacer::fill_v(1).build(),
                TextEdit::new(IdGen::literal("text_input_field"), "Potato".into())
                    .layout_hints(LayoutHints::fill_horizontal())
                    .padding(Vec2::new(3.0, 3.0))
                    .build(),
                BoxContainer::horizontal(
                    IdGen::key("buttons"),
                    vec![
                    Button::with_label("Add!")
                        .on_click(|state: &mut AppState| {
                            state.items.push(format!("Potato {}", state.items.len()));
                        })
                        .hints(LayoutHints::fill_horizontal())
                        .build(),
                    Button::with_label("Delete!")
                        .on_click(|state: &mut AppState| {
                            state.items.pop();
                        })
                        .hints(LayoutHints::fill_horizontal())
                        .build(),
                ])
                .layout_hints(LayoutHints::fill_horizontal())
                .build(),
            ],
        )
        .layout_hints(LayoutHints::fill())
        .build(),
    )
    .margin(Vec2::new(50.0, 50.0))
    .build()
}

fn main() {
    let mut ctx = Context::new();

    let screen_size = Vec2::new(800.0, 600.0);
    let screen_rect = Rect::from_min_size(Pos2::new(0.0, 0.0), screen_size);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Test GUI")
        .build(&event_loop)
        .unwrap();

    let mut painter = Painter::new(WgpuConfiguration::default(), 1, 0);
    unsafe { pollster::block_on(painter.set_window(Some(&window))).unwrap() };

    let mut state = AppState::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            winit::event::Event::MainEventsCleared => {
                ctx.run(&mut view(&state), &mut state);
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

                ctx.input_state.on_winit_event(&event);
            }
            _ => (),
        }
    })
}
