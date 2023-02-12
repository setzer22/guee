use std::{any::Any, cell::RefCell, ops::DerefMut};

use epaint::{ClippedPrimitive, Pos2, Rect, TessellationOptions, Vec2};

use crate::{
    callback::{AccessorRegistry, Callback, CallbackDispatch},
    input::InputState,
    memory::Memory,
    painter::Painter,
    theme::Theme,
    widget::DynWidget,
    widget_id::WidgetId,
};

pub struct Context {
    pub painter: RefCell<Painter>,
    pub input_state: InputState,
    pub accessor_registry: AccessorRegistry,
    pub dispatched_callbacks: RefCell<Vec<CallbackDispatch>>,
    pub memory: Memory,
    pub focus: RefCell<Option<WidgetId>>,
    pub theme: RefCell<Theme>,
}

impl Context {
    pub fn new(screen_size: Vec2) -> Self {
        Self {
            painter: RefCell::new(Painter::new()),
            input_state: InputState::new(screen_size),
            dispatched_callbacks: Default::default(),
            accessor_registry: Default::default(),
            memory: Default::default(),
            focus: Default::default(),
            theme: RefCell::new(Theme::new_empty()),
        }
    }
    pub fn run(&mut self, widget: &mut DynWidget, state: &mut dyn Any) {
        // Initialize a fresh painter
        self.painter.borrow_mut().prepare(
            Rect::from_min_size(Pos2::ZERO, self.input_state.screen_size),
            self.theme.borrow().text_color,
        );

        let mut layout = widget.widget.layout(
            self,
            WidgetId::new("__ROOT__"),
            self.input_state.screen_size,
        );
        layout.to_absolute(Vec2::ZERO);
        let events = std::mem::take(&mut self.input_state.ev_buffer);
        widget
            .widget
            // Pass list of events to on_event
            .on_event(
                self,
                &layout,
                self.input_state.mouse_state.position,
                &events,
            );
        widget.widget.draw(self, &layout);
        for callback in self.dispatched_callbacks.borrow_mut().drain(..) {
            self.accessor_registry.invoke_callback(state, callback);
        }
        self.input_state.end_frame();
    }

    pub fn tessellate(&mut self) -> Vec<ClippedPrimitive> {
        let mut painter = self.painter.borrow_mut();

        epaint::tessellate_shapes(
            1.0,
            TessellationOptions::default(),
            painter.fonts.font_image_size(),
            vec![],
            std::mem::take(&mut painter.shapes),
        )
    }

    pub fn dispatch_callback<P: 'static>(&self, c: Callback<P>, payload: P) {
        self.dispatched_callbacks
            .borrow_mut()
            .push(CallbackDispatch::new(c, payload))
    }

    pub fn request_focus(&self, widget_id: WidgetId) {
        *self.focus.borrow_mut() = Some(widget_id);
    }

    pub fn get_focus(&self) -> Option<WidgetId> {
        *self.focus.borrow()
    }

    pub fn is_focused(&self, widget_id: WidgetId) -> bool {
        self.focus.borrow().map(|x| widget_id == x).unwrap_or(false)
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = RefCell::new(theme);
    }

    /// Borrows the painter mutably.
    ///
    /// # Panics
    ///
    /// - When you request multiple painter borrows at the same time.
    /// - When the painter is not set, because there is no frame being rendered.
    pub fn painter(&self) -> impl DerefMut<Target = Painter> + '_ {
        self.painter.borrow_mut()
    }
}
