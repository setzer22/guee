use epaint::{Color32, Stroke};

pub trait Color32Ext: Sized + Copy {
    fn get_color(&mut self) -> &mut Color32;

    /// Multiplies the color by the given `value`. Keeps alpha as-is.
    fn lighten(self, value: f32) -> Self {
        let mut this = self;
        let color = this.get_color();
        let [mut r, mut g, mut b, a] = color.to_array().map(|x| x as f32 / u8::MAX as f32);
        r *= value;
        g *= value;
        b *= value;
        let [r, g, b, a] = [r, g, b, a].map(|x| (x * u8::MAX as f32) as u8);
        *color = Color32::from_rgba_premultiplied(r, g, b, a);
        this
    }
}

impl Color32Ext for Color32 {
    fn get_color(&mut self) -> &mut Color32 {
        self
    }
}

impl Color32Ext for Stroke {
    fn get_color(&mut self) -> &mut Color32 {
        &mut self.color
    }
}
