use epaint::{ahash::HashMap, Pos2, Vec2};
use winit::event::{ElementState, VirtualKeyCode, WindowEvent};

use crate::prelude::WidgetId;

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
    MouseWheel(Vec2),
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

#[derive(Copy, Clone, Debug, Default)]
pub enum ClickDragState {
    /// The mouse button isn't pressed
    #[default]
    Idle,
    /// The mouse button has been clicked, but hasn't moved enough distance
    Clicked(Pos2),
    /// The mouse button has moved enough with the mouse button held to register
    /// a drag and hasn't yet been released.
    Dragged(Pos2),
}

#[derive(Clone, Debug, Default)]
pub struct ButtonState {
    pub down: bool,
    pub drag_state: ClickDragState,
    pub just_pressed: bool,
    pub just_released: bool,
    // True during the frame after which the mouse is released, without having
    // moved a certain distance from where it was pressed (i.e. a 'click')
    pub just_clicked: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ButtonStateMap {
    state: HashMap<MouseButton, ButtonState>,
}

impl ButtonStateMap {
    /// Returns whether the mouse button is currently down
    pub fn is_down(&self, button: MouseButton) -> bool {
        self.state.get(&button).map(|x| x.down).unwrap_or(false)
    }

    /// Returns whether the mouse button has been pressed during this frame.
    pub fn is_pressed(&self, button: MouseButton) -> bool {
        self.state
            .get(&button)
            .map(|x| x.just_pressed)
            .unwrap_or(false)
    }

    /// Returns whether the mouse button has been released during this frame.
    pub fn is_released(&self, button: MouseButton) -> bool {
        self.state
            .get(&button)
            .map(|x| !x.just_released)
            .unwrap_or(false)
    }

    /// Returns whether the mouse button has been clicked during this frame.
    pub fn is_clicked(&self, button: MouseButton) -> bool {
        self.state
            .get(&button)
            .map(|x| x.just_clicked)
            .unwrap_or(false)
    }

    /// Returns the drag start position when the current `button` has currently
    /// started a drag event. None otherwise.
    pub fn is_dragging(&self, button: MouseButton) -> Option<Pos2> {
        self.state.get(&button).and_then(|x| match x.drag_state {
            ClickDragState::Idle => None,
            ClickDragState::Clicked(_) => None,
            ClickDragState::Dragged(pos) => Some(pos),
        })
    }

    /// Clears current "just pressed" state. Subsequent calls to
    /// on_mouse_pressed / on_mouse_released will activate the just_* flags
    fn end_frame(&mut self) {
        for (_, b_state) in self.state.iter_mut() {
            b_state.just_pressed = false;
            b_state.just_released = false;
            b_state.just_clicked = false;
        }
    }

    fn on_mouse_pressed(&mut self, button: MouseButton, cursor_pos: Pos2) {
        let entry = self.state.entry(button).or_default();
        entry.just_pressed = true;
        entry.down = true;
        entry.drag_state = ClickDragState::Clicked(cursor_pos);
    }

    pub fn on_mouse_released(&mut self, button: MouseButton) {
        let entry = self.state.entry(button).or_default();
        entry.just_released = true;
        entry.down = false;
        match entry.drag_state {
            ClickDragState::Clicked(_) => {
                entry.just_clicked = true;
            }
            ClickDragState::Idle => (),
            ClickDragState::Dragged(_) => (),
        }
        entry.drag_state = ClickDragState::Idle;
    }

    pub fn on_mouse_moved(&mut self, cursor_pos: Pos2) {
        const DRAG_THRESHOLD_PX: f32 = 4.0;
        for (_, b_state) in self.state.iter_mut() {
            match b_state.drag_state {
                ClickDragState::Idle => (),
                ClickDragState::Dragged(_) => (),
                ClickDragState::Clicked(pos) => {
                    if pos.distance(cursor_pos) > DRAG_THRESHOLD_PX {
                        b_state.drag_state = ClickDragState::Dragged(pos);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MouseState {
    pub position: Pos2,
    pub prev_position: Pos2,
    pub button_state: ButtonStateMap,
    /// If there's a current ongoing drag event, stores the position where the
    /// mouse started dragging from.
    pub ongoing_drag: ClickDragState,
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

#[derive(Clone, Debug, Default)]
pub struct InputWidgetState {
    pub focus: Option<WidgetId>,
    pub drag: Option<WidgetId>,
}

pub struct SenseResponse {
    drag: Option<Pos2>,
    clicked: bool,
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
        self.mouse_state.button_state.end_frame();
    }

    pub fn on_winit_event(&mut self, widget_state: &mut InputWidgetState, ev: &WindowEvent) {
        match ev {
            WindowEvent::CursorMoved { position, .. } => {
                let pos = Pos2::new(position.x as _, position.y as _);
                self.ev_buffer.push(Event::MouseMoved(pos));
                self.mouse_state.position = pos;
                self.mouse_state.button_state.on_mouse_moved(pos);
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
                        self.mouse_state
                            .button_state
                            .on_mouse_pressed(button, self.mouse_state.position);
                    }
                    ElementState::Released => {
                        self.ev_buffer.push(Event::MouseReleased(button));
                        self.mouse_state.button_state.on_mouse_released(button);
                        widget_state.drag = None;
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                const PIXELS_PER_LINE: f32 = 50.0;
                self.ev_buffer.push(Event::MouseWheel(match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => Vec2::new(*x, *y),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => Vec2::new(
                        pos.x as f32 * PIXELS_PER_LINE,
                        pos.y as f32 * PIXELS_PER_LINE,
                    ),
                }))
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
