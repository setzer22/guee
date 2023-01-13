use std::any::{Any, TypeId};

// A type-erased callback function
pub struct Callback {
    pub input_type: TypeId,
    #[allow(clippy::type_complexity)]
    pub f: Box<dyn FnOnce(&mut dyn Any)>,
}

impl Callback {
    pub fn from_fn<F, T>(f: F) -> Callback
    where
        T: 'static,
        F: FnOnce(&mut T) + 'static,
    {
        let closure = move |t_any: &mut dyn Any| {
            let t: &mut T = t_any.downcast_mut().expect("Failed downcast");
            f(t);
        };
        Callback {
            input_type: TypeId::of::<T>(),
            f: Box::new(closure),
        }
    }

    pub fn call(self, input: &mut dyn Any) {
        (self.f)(input);
    }
}

