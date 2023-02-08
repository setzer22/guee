use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
};

use epaint::ahash::HashMap;

use crate::widget_id::WidgetId;

#[derive(Default)]
pub struct Memory {
    pub widget_memory: RefCell<HashMap<WidgetId, Box<dyn Any>>>,
}

impl Memory {
    pub fn set<T: 'static>(&self, id: WidgetId, t: T) {
        self.widget_memory.borrow_mut().insert(id, Box::new(t));
    }

    pub fn ensure<T: 'static>(&self, id: WidgetId, t: T) {
        let contains = self.widget_memory.borrow().contains_key(&id);
        if !contains {
            self.set(id, t);
        }
    }

    pub fn ensure_default<T: Default + 'static>(&self, id: WidgetId) {
        let contains = self.widget_memory.borrow().contains_key(&id);
        if !contains {
            self.set(id, T::default());
        }
    }

    pub fn get<T: 'static>(&self, id: WidgetId) -> impl Deref<Target = T> + '_ {
        let mem = self.widget_memory.borrow();
        Ref::map(mem, |x| {
            x.get(&id)
                .expect("No value for given id")
                .downcast_ref::<T>()
                .expect("Failed downcast")
        })
    }

    #[track_caller]
    pub fn get_mut<T: 'static>(&self, id: WidgetId) -> impl DerefMut<Target = T> + '_ {
        let mem = self.widget_memory.borrow_mut();
        RefMut::map(mem, |x| {
            x.get_mut(&id)
                .unwrap()
                .downcast_mut::<T>()
                .expect("Failed downcast")
        })
    }

    pub fn get_or_default<T: Default + 'static>(&self, id: WidgetId) -> impl Deref<Target = T> + '_ {
        self.ensure_default::<T>(id);
        self.get(id)
    }

    pub fn get_mut_or_default<T: Default + 'static>(&self, id: WidgetId) -> impl DerefMut<Target = T> + '_ {
        self.ensure_default::<T>(id);
        self.get_mut(id)
    }

    pub fn get_or<T: 'static>(&self, id: WidgetId, t: T) -> impl Deref<Target = T> + '_ {
        self.ensure(id, t);
        self.get(id)
    }

    pub fn get_mut_or<T: 'static>(&self, id: WidgetId, t: T) -> impl DerefMut<Target = T> + '_ {
        self.ensure(id, t);
        self.get_mut(id)
    }
}
