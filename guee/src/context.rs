use std::{
    any::Any,
    cell::{Ref, RefCell},
    ops::{Deref, DerefMut},
};

use epaint::{ClippedPrimitive, Pos2, Rect, TessellationOptions, Vec2};

use crate::{
    callback::{
        AccessorRegistry, Callback, DispatchedCallbackStorage, DispatchedExternalCallback,
        PollToken,
    },
    input::InputState,
    memory::Memory,
    painter::{ExtraFont, Painter},
    theme::Theme,
    widget::DynWidget,
    widget_id::WidgetId,
};

pub struct Context {
    pub painter: RefCell<Painter>,
    pub input_state: InputState,
    pub accessor_registry: AccessorRegistry,
    pub dispatched_callbacks: RefCell<DispatchedCallbackStorage>,
    pub memory: Memory,
    pub focus: RefCell<Option<WidgetId>>,
    pub theme: RefCell<Theme>,
}

impl Context {
    pub fn new(screen_size: Vec2, extra_fonts: Vec<ExtraFont>) -> Self {
        Self {
            painter: RefCell::new(Painter::new(extra_fonts)),
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
        self.dispatched_callbacks
            .borrow_mut()
            .end_frame(state, &self.accessor_registry);
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
            .dispatch_callback(c, payload);
    }

    pub fn create_internal_callback<P: 'static>(&self) -> (Callback<P>, PollToken<P>) {
        self.dispatched_callbacks
            .borrow_mut()
            .create_internal_callback()
    }

    pub fn poll_callback_result<P: 'static>(
        &self,
        tk: PollToken<P>,
    ) -> Option<impl Deref<Target = P> + '_> {
        let guard = self.dispatched_callbacks.borrow();
        if guard.poll_callback_result(tk).is_some() {
            Some(Ref::map(guard, |x| x.poll_callback_result(tk).unwrap()))
        } else {
            None
        }
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
