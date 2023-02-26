use crate::{
    context::Context,
    input::{Event, EventStatus},
    layout::{Layout, LayoutHints, SizeHint, SizeHints},
    painter::{GueeGalley, GueeTextShape},
    widget::Widget,
    widget_id::WidgetId,
};
use epaint::{Color32, FontId, Pos2, Stroke, Vec2};
use guee_derives::Builder;

#[derive(Clone, Builder)]
#[builder(widget)]
pub struct Text {
    contents: String,
    #[builder(skip)]
    last_galley: Option<GueeGalley>,
    #[builder(default, strip_option)]
    color_override: Option<Color32>,
    #[builder(default = 14.0)]
    font_size: f32,
}

impl Text {
    pub fn ensure_galley(&mut self, ctx: &Context, wrap_width: f32) -> GueeGalley {
        let galley = ctx.painter().galley(
            self.contents.clone(),
            FontId::proportional(self.font_size),
            wrap_width,
        );
        self.last_galley = Some(galley.clone());
        galley
    }
}

impl Widget for Text {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        _force_shrink: bool, // ignore, always shrinked
    ) -> Layout {
        let galley = self.ensure_galley(ctx, available.x);
        Layout::leaf(parent_id.with(&self.contents), galley.bounds().size())
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let galley = self
            .last_galley
            .clone()
            .expect("Layout should be called before draw");
        ctx.painter().text_with_galley(GueeTextShape {
            galley,
            pos: layout.bounds.left_top(),
            underline: Stroke::NONE,
            angle: 0.0,
        });
    }

    fn layout_hints(&self) -> LayoutHints {
        LayoutHints {
            size_hints: SizeHints {
                width: SizeHint::Shrink,
                height: SizeHint::Shrink,
            },
            weight: 1,
        }
    }

    fn on_event(
        &mut self,
        _ctx: &Context,
        _layout: &Layout,
        _cursor_position: Pos2,
        _events: &[Event],
    ) -> EventStatus {
        EventStatus::Ignored
    }
}
