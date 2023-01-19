use epaint::{ahash::HashMap, Pos2};
use winit::event::{ElementState, VirtualKeyCode, WindowEvent};

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
                    ElementState::Pressed => {
                        self.ev_buffer.push(Event::MousePressed(button));
                        *entry = true;
                    }
                    ElementState::Released => {
                        self.ev_buffer.push(Event::MouseReleased(button));
                        *entry = false;
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
            _ => (),
        }
    }
}

/// Winit sends special keys (backspace, delete, F1, …) as characters.
/// Ignore those.
/// We also ignore '\r', '\n', '\t'.
/// Newlines are handled by the `Key::Enter` event.
fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = '\u{e000}' <= chr && chr <= '\u{f8ff}'
        || '\u{f0000}' <= chr && chr <= '\u{ffffd}'
        || '\u{100000}' <= chr && chr <= '\u{10fffd}';

    !is_in_private_use_area && !chr.is_ascii_control()
}
