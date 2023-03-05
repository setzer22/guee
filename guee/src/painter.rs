use std::sync::Arc;

use epaint::{
    emath::Align2,
    text::{FontData, FontDefinitions},
    CircleShape, ClippedShape, Color32, CubicBezierShape, FontFamily, FontId, Fonts, Galley, Mesh,
    Pos2, Rect, RectShape, Rounding, Stroke, TextShape, TextureId, Vec2,
};

pub struct Painter {
    pub clip_rect: Rect,
    pub text_color: Color32,
    pub shapes: Vec<ClippedShape>,
    pub overlay_shapes: Vec<ClippedShape>,
    pub transform: TranslateScale,
    pub use_overlay: bool,
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

pub struct ExtraFont {
    pub font_family: FontFamily,
    pub name: &'static str,
    pub data: &'static [u8],
}

#[allow(clippy::new_without_default)]
impl Painter {
    pub fn new(extra_fonts: Vec<ExtraFont>) -> Self {
        let mut font_defs = FontDefinitions::default();
        for (i, extra_font) in extra_fonts.into_iter().enumerate() {
            font_defs.font_data.insert(
                extra_font.name.to_owned(),
                FontData::from_static(extra_font.data),
            );
            font_defs
                .families
                .entry(epaint::FontFamily::Proportional)
                .or_default()
                .insert(i, extra_font.name.to_string())
        }

        Self {
            clip_rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            text_color: Color32::BLACK,
            shapes: Vec::new(),
            overlay_shapes: Vec::new(),
            transform: TranslateScale::identity(),
            use_overlay: false,
            fonts: Fonts::new(1.0, 1024, font_defs),
        }
    }

    pub fn prepare(&mut self, clip_rect: Rect, text_color: Color32) {
        self.clip_rect = clip_rect;
        self.text_color = text_color;
    }

    /// Sets the use of the overlay shape buffer. When enabled, shapes will be
    /// drawn on top of everything else.
    ///
    /// Returns the previously used overlay state. For easy restoration.
    pub fn set_overlay(&mut self, overlay: bool) -> bool {
        let prev = self.use_overlay;
        self.use_overlay = overlay;
        prev
    }

    pub fn with_overlay(&mut self, f: impl FnOnce(&mut Self)) {
        let old_overlay = self.use_overlay;
        self.use_overlay = true;
        f(self);
        self.use_overlay = old_overlay;
    }

    /// Pushes a shape to be drawn
    pub fn push_shape(&mut self, shape: epaint::Shape) {
        if self.use_overlay {
            self.overlay_shapes
                .push(ClippedShape(self.clip_rect, shape))
        } else {
            self.shapes.push(ClippedShape(self.clip_rect, shape))
        }
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
        self.push_shape(epaint::Shape::Rect(RectShape {
            rect: self.transform.transform_rectangle(rect),
            rounding: self.transform.transform_rounding(rounding),
            fill,
            stroke,
        }));
    }

    /// Paints the given `CircleShape`
    pub fn circle(&mut self, circle_shape: CircleShape) {
        let CircleShape {
            center,
            radius,
            fill,
            stroke,
        } = circle_shape;

        self.push_shape(epaint::Shape::Circle(CircleShape {
            center: self.transform.transform_point(center),
            radius: self.transform.transform_scalar(radius),
            fill,
            stroke,
        }));
    }

    /// Paints a tetured rect with the given texture_id with default UV mapping
    pub fn image(&mut self, rect: Rect, texture_id: TextureId) {
        let mut mesh = Mesh::with_texture(texture_id);
        mesh.add_rect_with_uv(
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        self.push_shape(epaint::Shape::mesh(mesh));
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
    pub fn text_with_galley(&mut self, text_shape: GueeTextShape) {
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
            let wrap_width = self.transform.transform_scalar(galley.wrap_width);
            GueeGalley {
                epaint_galley: self.fonts.layout(
                    galley.epaint_galley.job.text.clone(),
                    font_id.clone(),
                    Color32::BLACK, // Ignored
                    wrap_width,
                ),
                font_id,
                wrap_width: galley.wrap_width,
            }
        } else {
            galley
        };

        self.push_shape(epaint::Shape::Text(TextShape {
            pos: self.transform.transform_point(pos),
            override_text_color: Some(self.text_color),
            galley: galley.epaint_galley,
            underline,
            angle,
        }));
    }

    pub fn text(&mut self, pos: Pos2, align: Align2, label: impl ToString, font: FontId) {
        let galley = self.galley(label.to_string(), font, f32::INFINITY);
        let rect = align.anchor_rect(Rect::from_min_size(pos, galley.bounds().size()));
        self.text_with_galley(GueeTextShape {
            galley,
            pos: rect.min,
            underline: Stroke::NONE,
            angle: 0.0,
        })
    }

    pub fn line_segment(&mut self, points: [Pos2; 2], stroke: Stroke) {
        let mut points = points;
        let mut stroke = stroke;
        for point in &mut points {
            *point = self.transform.transform_point(*point);
        }
        stroke.width = self.transform.transform_scalar(stroke.width);

        self.push_shape(epaint::Shape::LineSegment { points, stroke })
    }

    pub fn cubic_bezier(&mut self, bezier_shape: CubicBezierShape) {
        let CubicBezierShape {
            mut points,
            closed,
            fill,
            mut stroke,
        } = bezier_shape;

        for point in &mut points {
            *point = self.transform.transform_point(*point);
        }
        stroke.width = self.transform.transform_scalar(stroke.width);

        self.push_shape(epaint::Shape::CubicBezier(CubicBezierShape {
            points,
            closed,
            fill,
            stroke,
        }))
    }

    /// Returns and drains the inner shape buffers. Use this method to draw the
    /// shapes, as it will handle the correct ordering
    pub fn take_shapes(&mut self) -> Vec<ClippedShape> {
        self.shapes.append(&mut self.overlay_shapes);
        std::mem::take(&mut self.shapes)
    }
}

/// A transformation consisting only of translation and uniform scaling
/// operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TranslateScale {
    translation: Vec2,
    scale: f32,
}

impl Default for TranslateScale {
    fn default() -> Self {
        Self::identity()
    }
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

    /// Applies the transformation in `other` after self. First scale,
    /// then translation.
    pub fn combined(&self, other: TranslateScale) -> TranslateScale {
        self.scaled(other.scale).translated(other.translation)
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
