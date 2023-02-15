use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
};

use epaint::ahash::HashMap;

use crate::widget_id::WidgetId;

#[derive(Default)]
pub struct Memory {
    // TODO: Cleanup old memory bits once they're no longer referenced
    pub widget_memory: RefCell<HashMap<(WidgetId, TypeId), Box<dyn Any>>>,
}

impl Memory {
    pub fn key<T: 'static>(id: WidgetId) -> (WidgetId, TypeId) {
        (id, TypeId::of::<T>())
    }

    pub fn set<T: 'static>(&self, id: WidgetId, t: T) {
        self.widget_memory
            .borrow_mut()
            .insert(Self::key::<T>(id), Box::new(t));
    }

    pub fn ensure<T: 'static>(&self, id: WidgetId, t: T) {
        let contains = self
            .widget_memory
            .borrow()
            .contains_key(&Self::key::<T>(id));
        if !contains {
            self.set(id, t);
        }
    }

    pub fn ensure_default<T: Default + 'static>(&self, id: WidgetId) {
        let contains = self
            .widget_memory
            .borrow()
            .contains_key(&Self::key::<T>(id));
        if !contains {
            self.set(id, T::default());
        }
    }

    pub fn get<T: 'static>(&self, id: WidgetId) -> impl Deref<Target = T> + '_ {
        let mem = self.widget_memory.borrow();
        Ref::map(mem, |x| {
            x.get(&Self::key::<T>(id))
                .expect("No value for given id")
                .downcast_ref::<T>()
                .expect("Failed downcast")
        })
    }

    #[track_caller]
    pub fn get_mut<T: 'static>(&self, id: WidgetId) -> impl DerefMut<Target = T> + '_ {
        let mem = self.widget_memory.borrow_mut();
        RefMut::map(mem, |x| {
            x.get_mut(&Self::key::<T>(id))
                .unwrap()
                .downcast_mut::<T>()
                .expect("Failed downcast")
        })
    }

    pub fn get_or_default<T: Default + 'static>(
        &self,
        id: WidgetId,
    ) -> impl Deref<Target = T> + '_ {
        self.ensure_default::<T>(id);
        self.get(id)
    }

    pub fn get_mut_or_default<T: Default + 'static>(
        &self,
        id: WidgetId,
    ) -> impl DerefMut<Target = T> + '_ {
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
