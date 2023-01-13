use epaint::{ahash::HashMap, Pos2};
use winit::event::WindowEvent;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}

#[derive(Clone, Debug)]
pub enum Event {
    MousePressed(MouseButton),
    MouseReleased(MouseButton),
    MouseMoved(Pos2),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EventStatus {
    Ignored,
    Consumed,
}

#[derive(Clone, Debug, Default)]
pub struct MouseState {
    pub position: Pos2,
    pub button_state: HashMap<MouseButton, bool>,
}

#[derive(Clone, Debug, Default)]
pub struct InputState {
    pub mouse_state: MouseState,
    pub ev_buffer: Vec<Event>,
}

impl InputState {
    pub fn on_winit_event(&mut self, ev: &WindowEvent) {
        match ev {
            WindowEvent::CursorMoved { position, .. } => {
                let pos = Pos2::new(position.x as _, position.y as _);
                self.ev_buffer.push(Event::MouseMoved(pos));
                self.mouse_state.position = pos;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let button = match button {
                    winit::event::MouseButton::Left => MouseButton::Primary,
                    winit::event::MouseButton::Right => MouseButton::Secondary,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    winit::event::MouseButton::Other(idx) => MouseButton::Other(*idx),
                };
                let entry = self.mouse_state.button_state.entry(button).or_default();
                match state {
                    winit::event::ElementState::Pressed => {
                        self.ev_buffer.push(Event::MousePressed(button));
                        *entry = true;
                    }
                    winit::event::ElementState::Released => {
                        self.ev_buffer.push(Event::MouseReleased(button));
                        *entry = false;
                    }
                }
            }
            _ => (),
        }
    }
}
