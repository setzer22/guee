use std::sync::Arc;

use epaint::{
    text::{FontDefinitions, LayoutJob},
    CircleShape, ClippedShape, Color32, FontId, Fonts, Galley, Pos2, Rect, RectShape, Rounding,
    Stroke, TextShape, Vec2,
};

pub struct Painter {
    pub clip_rect: Rect,
    pub text_color: Color32,
    pub shapes: Vec<ClippedShape>,
    pub transform: TranslateScale,
    pub fonts: Fonts,
}

/// Wraps an `epaint::galley`. This is necessary because epaint galleys don't
/// support scaling after they've been created, so as a workaround, we cache all
/// the parameters that were used to create the galley, so we can recreate it at
/// a different scale if it is rendererd with a custom transformation.
#[derive(Clone)]
pub struct GueeGalley {
    pub epaint_galley: Arc<Galley>,
    pub font_id: FontId,
    pub wrap_width: f32,
}

impl GueeGalley {
    pub fn bounds(&self) -> Rect {
        self.epaint_galley.rect
    }
}

pub struct GueeTextShape {
    pub galley: GueeGalley,
    pub pos: Pos2,
    pub underline: Stroke,
    pub angle: f32,
}

impl Painter {
    pub fn new() -> Self {
        Self {
            clip_rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            text_color: Color32::BLACK,
            shapes: Vec::new(),
            transform: TranslateScale::identity(),
            fonts: Fonts::new(1.0, 1024, FontDefinitions::default()),
        }
    }

    pub fn prepare(&mut self, clip_rect: Rect, text_color: Color32) {
        self.clip_rect = clip_rect;
        self.text_color = text_color;
    }

    /// Paints the given `RectShape`
    pub fn rect(&mut self, rect_shape: RectShape) {
        // Apply current transformation to shape
        let RectShape {
            rect,
            rounding,
            fill,
            stroke,
        } = rect_shape;
        self.shapes.push(ClippedShape(
            self.clip_rect,
            epaint::Shape::Rect(RectShape {
                rect: self.transform.transform_rectangle(rect),
                rounding: self.transform.transform_rounding(rounding),
                fill,
                stroke,
            }),
        ));
    }

    /// Paints the given `CircleShape`
    pub fn circle(&mut self, circle_shape: CircleShape) {
        let CircleShape {
            center,
            radius,
            fill,
            stroke,
        } = circle_shape;

        self.shapes.push(ClippedShape(
            self.clip_rect,
            epaint::Shape::Circle(CircleShape {
                center: self.transform.transform_point(center),
                radius: self.transform.transform_scalar(radius),
                fill,
                stroke,
            }),
        ));
    }

    pub fn galley(&mut self, contents: String, font_id: FontId, wrap_width: f32) -> GueeGalley {
        GueeGalley {
            epaint_galley: self.fonts.layout(
                contents,
                font_id.clone(),
                Color32::BLACK, // Ignored
                wrap_width,
            ),
            font_id,
            wrap_width,
        }
    }

    /// Paints the given `TextShape`.
    ///
    /// ## Text color
    ///
    /// Note that the provided text colors, both in the galley and the override
    /// are ignored by this function, and instead the `text_color` stored in the
    /// painter property is used.
    pub fn text(&mut self, text_shape: GueeTextShape) {
        let GueeTextShape {
            galley,
            pos,
            underline,
            angle,
        } = text_shape;

        // Only redo the layout job if there is scale
        let galley = if self.transform.scale != 1.0 {
            let mut font_id = galley.font_id.clone();
            font_id.size = self.transform.transform_scalar(font_id.size);
            GueeGalley {
                epaint_galley: self.fonts.layout(
                    galley.epaint_galley.job.text.clone(),
                    font_id,
                    Color32::BLACK, // Ignored
                    self.transform.transform_scalar(galley.wrap_width)
                ),
                font_id: galley.font_id,
                wrap_width: galley.wrap_width,
            }
        } else {
            galley
        };

        self.shapes.push(ClippedShape(
            self.clip_rect,
            epaint::Shape::Text(TextShape {
                pos: self.transform.transform_point(pos),
                override_text_color: Some(self.text_color),
                galley: galley.epaint_galley,
                underline,
                angle,
            }),
        ));
    }
}

/// A transformation consisting only of translation and uniform scaling
/// operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TranslateScale {
    translation: Vec2,
    scale: f32,
}

impl TranslateScale {
    /// Returns the identity transformation.
    pub fn identity() -> Self {
        Self {
            translation: Vec2::new(0.0, 0.0),
            scale: 1.0,
        }
    }

    /// Returns a new transformation, translated by the given vector
    pub fn translated(&self, translation: Vec2) -> Self {
        Self {
            translation: self.translation + translation,
            scale: self.scale,
        }
    }

    /// Returns a new transformation, scaled by the given amount
    pub fn scaled(&self, scale: f32) -> Self {
        let new_scale = self.scale * scale;
        Self {
            translation: self.translation * scale,
            scale: new_scale,
        }
    }

    /// Applies the scaling and translation of this transformation to the given
    /// `point`.
    pub fn transform_point(&self, point: Pos2) -> Pos2 {
        Pos2::new(point.x * self.scale, point.y * self.scale) + self.translation
    }

    /// Applies the scaling of this transformation to the given `scalar`.
    /// Translation is ignored.
    pub fn transform_scalar(&self, s: f32) -> f32 {
        s * self.scale
    }

    /// Applies the scaling of this transformation to the given `rounding`.
    /// Translation is ignored.
    pub fn transform_rounding(&self, s: Rounding) -> Rounding {
        Rounding {
            nw: self.transform_scalar(s.nw),
            ne: self.transform_scalar(s.ne),
            sw: self.transform_scalar(s.sw),
            se: self.transform_scalar(s.se),
        }
    }

    /// Applies the scaling and translation of this transformation to the given
    /// `rectangle`. The rectangles's dimensions may be set to infinity.
    pub fn transform_rectangle(&self, rectangle: Rect) -> Rect {
        let top_left = self.transform_point(rectangle.left_top());
        let size = Vec2::new(
            rectangle.width() * self.scale,
            rectangle.height() * self.scale,
        );
        Rect::from_min_size(top_left, size)
    }
}
