use std::{any::Any, borrow::BorrowMut, cell::RefCell, ops::DerefMut};

use epaint::{ClippedPrimitive, Pos2, Rect, TessellationOptions, Vec2};

use crate::{
    callback::{Callback, DispatchedCallbackStorage, PollToken},
    input::{InputState, InputWidgetState, MouseButton},
    memory::Memory,
    painter::{ExtraFont, Painter},
    theme::Theme,
    widget::DynWidget,
    widget_id::WidgetId,
};

pub struct Context {
    pub painter: RefCell<Painter>,
    pub input_state: InputState,
    pub input_widget_state: RefCell<InputWidgetState>,
    pub dispatched_callbacks: RefCell<DispatchedCallbackStorage>,
    pub memory: Memory,
    pub theme: RefCell<Theme>,
}

impl Context {
    /// Creates a new [`Context`]. The context object holds all the necessary
    /// state to render a UI using `guee`.o
    ///
    /// The Context object makes use of interior mutability. Many of its &self
    /// methods will modify its internal state.
    pub fn new(screen_size: Vec2, extra_fonts: Vec<ExtraFont>) -> Self {
        Self {
            painter: RefCell::new(Painter::new(extra_fonts)),
            input_state: InputState::new(screen_size),
            dispatched_callbacks: Default::default(),
            memory: Default::default(),
            input_widget_state: Default::default(),
            theme: RefCell::new(Theme::new_empty()),
        }
    }

    /// Draws the provided `widget` tree. To get the results, call
    /// [`Context::tessellate`]
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
            false,
        );
        layout.to_absolute(Vec2::ZERO);
        let events = std::mem::take(&mut self.input_state.ev_buffer);
        widget
            .widget
            // Pass list of events to on_event
            .on_event(self, &layout, self.input_state.mouse.position, &events);
        widget.widget.draw(self, &layout);
        self.dispatched_callbacks.borrow_mut().end_frame(state);
        self.input_state
            .end_frame(&mut self.input_widget_state.borrow_mut());
    }

    /// Returns a list of [`ClippedPrimitive`], suitable for rendering with an
    /// egui-compatible renderer.
    pub fn tessellate(&mut self) -> Vec<ClippedPrimitive> {
        let mut painter = self.painter.borrow_mut();

        epaint::tessellate_shapes(
            1.0,
            TessellationOptions::default(),
            painter.fonts.font_image_size(),
            vec![],
            painter.borrow_mut().take_shapes(),
        )
    }

    pub fn on_winit_event(&mut self, event: &winit::event::WindowEvent) {
        self.input_state
            .on_winit_event(self.input_widget_state.get_mut(), event);
    }

    /// Typically called from within widget code. Signals that the given
    /// callback `c` has been fired.
    pub fn dispatch_callback<P: 'static>(&self, c: Callback<P>, payload: P) {
        self.dispatched_callbacks
            .borrow_mut()
            .dispatch_callback(c, payload);
    }

    /// Typically called from within widget code. Allocates a new polling-based
    /// internal callback and returns it, together with its `PollToken`. See
    /// documentation on `Callback` for an explanation on internal callbacks.
    pub fn create_internal_callback<P: 'static>(&self) -> (Callback<P>, PollToken<P>) {
        self.dispatched_callbacks
            .borrow_mut()
            .create_internal_callback()
    }

    /// Given the `PollToken` for a callback previously allocated via
    /// `Context::create_internal_callback`, tries to fetch the result (if the
    /// callback was fired) and returns it.
    ///
    /// Note that calling this function takes ownership of the payload object,
    /// and subsequent calls to this function with the same token will always
    /// return None.
    pub fn poll_callback_result<P: 'static>(&self, tk: PollToken<P>) -> Option<P> {
        self.dispatched_callbacks
            .borrow_mut()
            .poll_callback_result(tk)
    }

    /// Requests focus for the given `widget_id`. The context will keep track of
    /// this widget being the focused one until some other widget calls this
    /// function, or the [`Context::release_focus`] function is called.
    pub fn request_focus(&self, widget_id: WidgetId) {
        self.input_widget_state.borrow_mut().focus = Some(widget_id);
    }

    /// Releases the focus for the given `widget_id`. If the given id does not
    /// match the currently focused widget, does nothing.
    pub fn release_focus(&self, widget_id: WidgetId) {
        let mut state = self.input_widget_state.borrow_mut();
        if let Some(id) = state.focus {
            if id == widget_id {
                state.focus = None;
            }
        }
    }

    /// Returns the currently focused widget, if any.
    pub fn get_focus(&self) -> Option<WidgetId> {
        self.input_widget_state.borrow().focus
    }

    /// Returns whether the given `widget_id` is the currently focused widget.
    pub fn is_focused(&self, widget_id: WidgetId) -> bool {
        self.input_widget_state
            .borrow()
            .focus
            .map(|x| widget_id == x)
            .unwrap_or(false)
    }

    /// If there is an ongoing mouse drag event inside `rect`, and no other
    /// widget claimed this drag event before, registers the given `widget_id`
    /// as the widget that is currently handling that event.
    ///
    /// This function takes into account the cursor scaling set inside
    /// `InputWidgetState`. Thus, widgets can send possibly transformed mouse
    /// coordinates set by their parents.
    ///
    /// The drag event can only be claimed when the drag position is inside the.
    /// But successive calls to this function after teh drag event has been
    /// claimed will continue to return true until the drag event ends.
    pub fn claim_drag_event(
        &self,
        widget_id: WidgetId,
        rect: Rect,
        mouse_button: MouseButton,
    ) -> bool {
        let mut wstate = self.input_widget_state.borrow_mut();
        let drag = self
            .input_state
            .mouse
            .button_state
            .is_dragging(mouse_button);

        if let Some(drag_widget) = wstate.drag {
            if drag_widget == widget_id {
                return drag.is_some();
            }
        } else if let Some(drag_pos) = drag {
            // Handle scaling, where layout is untransformed but mouse positions
            // and painter shapes are.
            let transformed_pos = wstate.cursor_transform.transform_point(drag_pos);

            if rect.contains(transformed_pos) {
                wstate.drag = Some(widget_id);
                return true;
            }
        }
        false
    }

    /// Sets the theme for this context to the given `theme`.
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
