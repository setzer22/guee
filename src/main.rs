use std::sync::Arc;

use base_widgets::{box_container::BoxContainer, button::Button, text::Text};
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

//pub mod epaint_shape_routine;
pub mod epaint_routine;

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
    BoxContainer::vertical()
        .contents(vec![
            BoxContainer::vertical()
                .contents(
                    state
                        .items
                        .iter()
                        .map(|it| Text::builder().contents(it.clone()).build().to_dyn())
                        .collect_vec(),
                )
                .build()
                .to_dyn(),
            BoxContainer::horizontal()
                .contents(vec![
                    Button::builder()
                        .contents(Text::builder().contents("Add!".into()).build().to_dyn())
                        .build()
                        .on_click(|state: &mut AppState| {
                            state.items.push(format!("Potato {}", state.items.len()));
                        })
                        .to_dyn(),
                    Button::builder()
                        .contents(Text::builder().contents("Delete!".into()).build().to_dyn())
                        .build()
                        .on_click(|state: &mut AppState| {
                            state.items.pop();
                        })
                        .to_dyn(),
                ])
                .build()
                .to_dyn(),
        ])
        .build()
        .to_dyn()
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
