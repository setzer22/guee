use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::prelude::Widget;

pub trait StyledWidget: Widget {
    type Style;
}

pub struct Theme {
    widget_styles: HashMap<TypeId, Box<dyn Any>>,
}

impl Theme {
    pub fn new_empty() -> Self {
        Theme {
            widget_styles: Default::default(),
        }
    }

    pub fn set_style<W>(&mut self, style: W::Style)
    where
        W: StyledWidget + Sized + 'static,
        W::Style: Sized + 'static,
    {
        self.widget_styles
            .insert(TypeId::of::<W>(), Box::new(style));
    }

    pub fn get_style<W>(&self) -> Option<&W::Style>
    where
        W: StyledWidget + Sized + 'static,
        W::Style: Sized + 'static,
    {
        self.widget_styles.get(&TypeId::of::<W>()).map(|x| {
            x.downcast_ref::<W::Style>()
                .expect("Downcast failed: Should contain the right style type")
        })
    }
}
