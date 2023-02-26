use guee_derives::Builder;

use crate::{painter::Painter, prelude::*};

// type-alias-impl-trait unfortunately no go brrrr yet, so we do this instead
macro_rules! event_fn_ty {
    (boxed) => {
        Box<event_fn_ty!(inner dyn)>
    };
    (generic) => {
        event_fn_ty!(inner impl)
    };
    (inner $token:tt) => {
        $token FnOnce(&Context, &Layout, Pos2, &[Event]) -> EventStatus + 'static
    };
}
macro_rules! draw_fn_ty {
    (boxed) => {
        Box<draw_fn_ty!(inner dyn)>
    };
    (generic) => {
        draw_fn_ty!(inner impl)
    };
    (inner $token:tt) => {
        $token FnOnce(&Context, &Layout) + 'static
    };
}
macro_rules! layout_fn_ty {
    (boxed) => {
        Box<layout_fn_ty!(inner dyn)>
    };
    (generic) => {
        layout_fn_ty!(inner impl)
    };
    (inner $token:tt) => {
        $token FnOnce(&Context, &Layout) + 'static
    };
}

/// A container that lets you wrap another widget and add custom code at
/// different points of the widget lifecycle.
///
/// This container should cover the majority of use cases in which one would
/// instead use a custom widget, such as custom drawing, custom input handling
/// and a few other things.
#[derive(Builder)]
#[builder(widget)]
#[allow(clippy::type_complexity)]
pub struct TinkerContainer {
    pub contents: DynWidget,
    #[builder(skip)]
    pub pre_event: Option<event_fn_ty!(boxed)>,
    #[builder(skip)]
    pub post_event: Option<event_fn_ty!(boxed)>,
    #[builder(skip)]
    pub pre_draw: Option<draw_fn_ty!(boxed)>,
    #[builder(skip)]
    pub post_draw: Option<draw_fn_ty!(boxed)>,
    #[builder(skip)]
    pub post_layout: Option<layout_fn_ty!(boxed)>,
}

impl TinkerContainer {
    /// Injects code before this widget's `on_event` callback. If the event is
    /// consumed by another widget before this one in the tree, this won't be
    /// called.
    ///
    /// The returned EventStatus can be used to stop event propagation.
    pub fn pre_event(mut self, f: event_fn_ty!(generic)) -> Self {
        self.pre_event = Some(Box::new(f));
        self
    }

    /// Injects code after this widget's `on_event` callback. If the event is
    /// consumed by other widgets before this one in the tree, or by its
    /// `contents`, this won't be called.
    ///
    /// The returned EventStatus can be used to stop event propagation.
    pub fn post_event(mut self, f: event_fn_ty!(generic)) -> Self {
        self.post_event = Some(Box::new(f));
        self
    }

    /// Injects code before this widget's draw callback.
    pub fn pre_draw(mut self, f: draw_fn_ty!(generic)) -> Self {
        self.pre_draw = Some(Box::new(f));
        self
    }

    /// Injects code after this widget's draw callback.
    pub fn post_draw(mut self, f: draw_fn_ty!(generic)) -> Self {
        self.post_draw = Some(Box::new(f));
        self
    }

    /// Injects code after this widget's layout has been computed.
    pub fn post_layout(mut self, f: layout_fn_ty!(generic)) -> Self {
        self.post_layout = Some(Box::new(f));
        self
    }
}

impl Widget for TinkerContainer {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let layout = self
            .contents
            .widget
            .layout(ctx, parent_id, available, force_shrink);

        if let Some(post_layout) = self.post_layout.take() {
            (post_layout)(ctx, &layout);
        }

        layout
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        if let Some(pre_draw) = self.pre_draw.take() {
            (pre_draw)(ctx, layout);
        }
        self.contents.widget.draw(ctx, layout);
        if let Some(post_draw) = self.post_draw.take() {
            (post_draw)(ctx, layout);
        }
    }

    fn layout_hints(&self) -> LayoutHints {
        self.contents.widget.layout_hints()
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
    ) -> EventStatus {
        if let Some(pre) = self.pre_event.take() {
            if let EventStatus::Consumed = (pre)(ctx, layout, cursor_position, events) {
                return EventStatus::Consumed;
            }
        }

        if let EventStatus::Consumed =
            self.contents
                .widget
                .on_event(ctx, layout, cursor_position, events)
        {
            return EventStatus::Consumed;
        }

        if let Some(post) = self.post_event.take() {
            (post)(ctx, layout, cursor_position, events)
        } else {
            EventStatus::Ignored
        }
    }
}
