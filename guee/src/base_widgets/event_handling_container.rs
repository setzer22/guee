use guee_derives::Builder;

use crate::prelude::*;

// type-alias-impl-trait unfortunately no go brrrr yet, so we do this instead
macro_rules! fn_ty {
    (boxed) => {
        Box<fn_ty!(inner dyn)>
    };
    (generic) => {
        fn_ty!(inner impl)
    };
    (inner $token:tt) => {
        $token FnOnce(&Context, &Layout, Pos2, &[Event]) -> EventStatus + 'static
    };
}

/// A container that forces to render its child widget with a specific maximum size.
#[derive(Builder)]
#[builder(widget)]
#[allow(clippy::type_complexity)]
pub struct EventHandlingContainer {
    pub contents: DynWidget,
    /// Takes the context and the list of events. If the event status is
    /// returned
    #[builder(skip)]
    pub pre_event: Option<fn_ty!(boxed)>,
    #[builder(skip)]
    pub post_event: Option<fn_ty!(boxed)>,
}

impl EventHandlingContainer {
    pub fn pre_event(mut self, f: fn_ty!(generic)) -> Self {
        self.pre_event = Some(Box::new(f));
        self
    }

    pub fn post_event(mut self, f: fn_ty!(generic)) -> Self {
        self.post_event = Some(Box::new(f));
        self
    }
}

impl Widget for EventHandlingContainer {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        self.contents
            .widget
            .layout(ctx, parent_id, available, force_shrink)
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        self.contents.widget.draw(ctx, layout);
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
