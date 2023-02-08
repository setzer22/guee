use epaint::{ClippedShape, Color32, Rect, RectShape, TextShape};

#[derive(Debug, Clone)]
pub struct Painter {
    pub clip_rect: Rect,
    pub text_color: Color32,
    pub shapes: Vec<ClippedShape>,
}

impl Painter {
    /// Paints the given `RectShape`
    pub fn rect(&mut self, rect: RectShape) {
        self.shapes
            .push(ClippedShape(self.clip_rect, epaint::Shape::Rect(rect)));
    }

    /// Paints the given `TextShape`.
    ///
    /// ## Text color
    ///
    /// Note that the provided text colors, both in the galley and the override
    /// are ignored by this function, and instead the `text_color` stored in the
    /// painter property is used.
    pub fn text(&mut self, text: TextShape) {
        self.shapes.push(ClippedShape(
            self.clip_rect,
            epaint::Shape::Text(TextShape {
                override_text_color: Some(self.text_color),
                ..text
            }),
        ));
    }
}
