use std::iter::repeat;

use epaint::{emath::Align2, RectShape, Rounding};
use guee_derives::Builder;

use crate::{callback::PollToken, input::MouseButton, prelude::*};

#[derive(Builder)]
#[builder(widget)]
pub struct MenubarButton {
    pub id: IdGen,
    pub label: String,
    pub button_options: Vec<String>,
    #[builder(strip_option)]
    pub on_option_selected: Option<Callback<usize>>,
    #[builder(default)]
    pub layout_hints: LayoutHints,
    #[builder(skip)]
    pub inner_widgets: Option<InnerWidgets>,
    #[builder(default = Vec2::new(2.0, 5.0))]
    pub inner_padding: Vec2,
    #[builder(default)]
    pub menu_min_width: f32,
    #[builder(default)]
    pub button_icons: Vec<(TextureId, Rect)>,
    #[builder(default = Vec2::new(16.0, 16.0))]
    pub icon_size: Vec2,
}

pub struct InnerWidgets {
    pub outer_button: DynWidget,
    pub inner_contents: DynWidget,
    pub inner_poll_tokens: Vec<PollToken<()>>,
    pub outer_poll_token: PollToken<()>,
}

pub struct MenubarButtonState {
    is_open: bool,
}

#[derive(Builder, Default, Clone)]
pub struct MenubarButtonStyle {
    pub outer_button: ButtonStyle,
    pub inner_button: ButtonStyle,
    pub menu_fill: Color32,
    pub menu_stroke: Stroke,
}

impl Widget for MenubarButton {
    fn layout(
        &mut self,
        ctx: &Context,
        parent_id: WidgetId,
        available: Vec2,
        force_shrink: bool,
    ) -> Layout {
        let widget_id = self.id.resolve(parent_id);

        let padding = Vec2::new(10.0, 2.0);

        // Initialize the inner widgets and set up internal callbacks for them
        if self.inner_widgets.is_none() {
            let default_theme = MenubarButtonStyle::default();
            let theme = ctx.theme.borrow();
            let theme = theme.get_style::<Self>().unwrap_or(&default_theme);

            let (inner_cbs, inner_poll_tokens): (Vec<Callback<()>>, Vec<PollToken<()>>) =
                (0..self.button_options.len())
                    .map(|_| ctx.create_internal_callback())
                    .unzip();
            let (outer_cb, outer_poll_token) = ctx.create_internal_callback();

            self.inner_widgets = Some(InnerWidgets {
                outer_button: Button::with_label(&self.label)
                    .padding(padding)
                    .style_override(theme.outer_button.clone())
                    .on_click(outer_cb)
                    .build(),
                inner_contents: MarginContainer::new(
                    IdGen::key("contents"),
                    BoxContainer::vertical(
                        IdGen::key("contents_v"),
                        self.button_options
                            .iter()
                            .zip(
                                // Add the button icons
                                self.button_icons.iter().map(Some).chain(repeat(None)),
                            )
                            .zip(inner_cbs.into_iter())
                            .map(|((s, ico), cb)| {
                                let button = if let Some((tex_id, uv_rect)) = ico {
                                    Button::with_icon_and_label(
                                        s,
                                        *tex_id,
                                        *uv_rect,
                                        self.icon_size,
                                    )
                                } else {
                                    Button::with_label(s)
                                };
                                button
                                    .on_click(cb)
                                    .padding(padding)
                                    .align_contents(Align2::LEFT_CENTER)
                                    .style_override(theme.inner_button.clone())
                                    .hints(LayoutHints::fill_horizontal())
                                    .min_size(Vec2::new(self.menu_min_width, 0.0))
                                    .build()
                            })
                            .collect(),
                    )
                    .build(),
                )
                .margin(self.inner_padding)
                .build(),
                inner_poll_tokens,
                outer_poll_token,
            })
        }

        let is_open = ctx
            .memory
            .get_or(widget_id, MenubarButtonState { is_open: false })
            .is_open;

        let mut children = Vec::new();

        let inner_widgets = self.inner_widgets.as_mut().unwrap();

        let outer_button_layout =
            inner_widgets
                .outer_button
                .widget
                .layout(ctx, widget_id, available, force_shrink);
        let outer_button_bounds = outer_button_layout.bounds;
        children.push(outer_button_layout);

        if is_open {
            let inner_contents_layout = inner_widgets
                .inner_contents
                .widget
                .layout(ctx, widget_id, available, force_shrink)
                .translated((outer_button_bounds.left_bottom() + Vec2::new(0.0, 3.0)).to_vec2());

            children.push(inner_contents_layout);
        }

        Layout::with_children(widget_id, outer_button_bounds.size(), children)
    }

    fn draw(&mut self, ctx: &Context, layout: &Layout) {
        let inner_widgets = self.inner_widgets.as_mut().unwrap();

        inner_widgets
            .outer_button
            .widget
            .draw(ctx, &layout.children[0]);

        let state = ctx.memory.get::<MenubarButtonState>(layout.widget_id);
        if state.is_open && layout.children.len() > 1 {
            let prev_overlay = ctx.painter().set_overlay(true);

            let theme = ctx.theme.borrow();
            let theme = theme.get_style::<Self>();

            ctx.painter().rect(RectShape {
                rect: layout.children[1].bounds.translate(Vec2::new(3.0, 2.0)),
                rounding: Rounding::same(2.0),
                fill: color!("#00000033"),
                stroke: Stroke::NONE,
            });

            ctx.painter().rect(RectShape {
                rect: layout.children[1].bounds,
                rounding: Rounding::same(2.0),
                fill: theme.map(|x| x.menu_fill).unwrap_or(color!("#191919")),
                stroke: theme
                    .map(|x| x.menu_stroke)
                    .unwrap_or(Stroke::new(1.0, color!("#dddddd"))),
            });

            inner_widgets
                .inner_contents
                .widget
                .draw(ctx, &layout.children[1]);

            ctx.painter().set_overlay(prev_overlay);
        }
    }

    fn layout_hints(&self) -> LayoutHints {
        self.layout_hints
    }

    fn on_event(
        &mut self,
        ctx: &Context,
        layout: &Layout,
        cursor_position: Pos2,
        events: &[Event],
        status: &mut EventStatus,
    ) {
        let inner_widgets = self.inner_widgets.as_mut().unwrap();
        inner_widgets.outer_button.widget.on_event(
            ctx,
            &layout.children[0],
            cursor_position,
            events,
            &mut EventStatus::Ignored, // Don't let inner widgets consume events
        );

        if ctx
            .poll_callback_result(inner_widgets.outer_poll_token)
            .is_some()
        {
            let mut state = ctx.memory.get_mut::<MenubarButtonState>(layout.widget_id);
            state.is_open = true;
            status.consume_event();
        }

        if ctx
            .memory
            .get::<MenubarButtonState>(layout.widget_id)
            .is_open
            && layout.children.len() > 1
        {
            inner_widgets.inner_contents.widget.on_event(
                ctx,
                &layout.children[1],
                cursor_position,
                events,
                &mut EventStatus::Ignored, // Don't let inner widgets consume events
            );

            for (idx, tk) in inner_widgets.inner_poll_tokens.iter().copied().enumerate() {
                if ctx.poll_callback_result(tk).is_some() {
                    ctx.memory
                        .get_mut::<MenubarButtonState>(layout.widget_id)
                        .is_open = false;
                    if let Some(on_option_selected) = self.on_option_selected.take() {
                        ctx.dispatch_callback(on_option_selected, idx);
                        status.consume_event();
                    }
                }
            }
        }

        // Dismiss click detection
        {
            let mut state = ctx.memory.get_mut::<MenubarButtonState>(layout.widget_id);
            let mouse_pos = cursor_position;
            if state.is_open {
                if ctx
                    .input_state
                    .mouse
                    .button_state
                    .is_clicked(MouseButton::Primary)
                    && !layout.children[0].bounds.contains(mouse_pos)
                    && !layout.children[1].bounds.contains(mouse_pos)
                {
                    state.is_open = false;
                }
            }
        }
    }
}

impl StyledWidget for MenubarButton {
    type Style = MenubarButtonStyle;
}
