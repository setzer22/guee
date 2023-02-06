use std::ops::Index;

use epaint::{ahash::HashMap, Pos2, Vec2};
use winit::event::{ElementState, VirtualKeyCode, WindowEvent};

use crate::widget_id::WidgetId;

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
    Text(char),
    KeyPressed(VirtualKeyCode),
    KeyReleased(VirtualKeyCode),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EventStatus {
    Ignored,
    Consumed,
}

impl EventStatus {
    pub fn or_else(&self, f: impl FnOnce() -> EventStatus) -> Self {
        match self {
            EventStatus::Ignored => f(),
            EventStatus::Consumed => EventStatus::Consumed,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ButtonState {
    state: HashMap<MouseButton, bool>,
}

impl ButtonState {
    pub fn is_down(&self, button: MouseButton) -> bool {
        *self.state.get(&button).unwrap_or(&false)
    }

    pub fn register_down(&mut self, button: MouseButton) {
        *self.state.entry(button).or_default() = true;
    }

    pub fn register_up(&mut self, button: MouseButton) {
        *self.state.entry(button).or_default() = false;
    }
}

#[derive(Clone, Debug, Default)]
pub struct MouseState {
    pub position: Pos2,
    pub prev_position: Pos2,
    pub button_state: ButtonState,
}

impl MouseState {
    pub fn delta(&self) -> Vec2 {
        self.position - self.prev_position
    }
}

#[derive(Clone, Debug)]
pub struct InputState {
    pub screen_size: Vec2,
    pub mouse_state: MouseState,
    pub ev_buffer: Vec<Event>,
}

impl InputState {
    pub fn new(screen_size: Vec2) -> Self {
        Self {
            screen_size,
            mouse_state: Default::default(),
            ev_buffer: Default::default(),
        }
    }

    pub fn end_frame(&mut self) {
        self.mouse_state.prev_position = self.mouse_state.position;
    }

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
                match state {
                    ElementState::Pressed => {
                        self.ev_buffer.push(Event::MousePressed(button));
                        self.mouse_state.button_state.register_down(button);
                    }
                    ElementState::Released => {
                        self.ev_buffer.push(Event::MouseReleased(button));
                        self.mouse_state.button_state.register_up(button);
                    }
                }
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    match input.state {
                        ElementState::Pressed => self.ev_buffer.push(Event::KeyPressed(keycode)),
                        ElementState::Released => self.ev_buffer.push(Event::KeyReleased(keycode)),
                    }
                }
            }
            WindowEvent::ReceivedCharacter(ch) => {
                // WIP: This is getting very complex very fast. Would it make
                // sense to bring egui's input management code here? At the very
                // least, their egui integration code in egui-winit looks like
                // something to take inspiration from.
                //
                // WIP2: There's also an issue with on_event. It currently gets
                // called once for each event, whereas it would be better if
                // it's called with all events at the same time.
                if is_printable_char(*ch) {
                    self.ev_buffer.push(Event::Text(*ch));
                }
            }
            WindowEvent::Resized(new_size) => {
                self.screen_size = Vec2::new(new_size.width as f32, new_size.height as f32);
            }
            _ => (),
        }
    }
}

/// Winit sends special keys (backspace, delete, F1, …) as characters.
/// Ignore those.
/// We also ignore '\r', '\n', '\t'.
/// Newlines are handled by the `Key::Enter` event.
fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
        || ('\u{100000}'..='\u{10fffd}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}
