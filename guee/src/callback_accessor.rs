use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use dyn_clone::{clone_trait_object, DynClone};

use crate::{callback::ExternalCallback, prelude::Callback};

pub trait AccessorFn: DynClone {
    fn call<'a>(&self, r: &'a mut dyn Any) -> &'a mut dyn Any;
}

clone_trait_object!(AccessorFn);

impl<F> AccessorFn for F
where
    F: (Fn(&mut dyn Any) -> &mut dyn Any) + Clone,
{
    fn call<'a>(&self, r: &'a mut dyn Any) -> &'a mut dyn Any {
        (self)(r)
    }
}

/// A `CallbackAccessor` offers an easy way to create callbacks that take some
/// subset `T` of the app's root state. It does that, by internally storing a
/// function that takes a mutable reference to the root state and returns a
/// mutable reference to some of this fields.
pub struct CallbackAccessor<T> {
    /// A function which takes the type-erased root state, and returns a `T`
    /// value. The root state type is type-erased because we don't want the user
    accessor_fns: Vec<Box<dyn AccessorFn>>,
    _phantom: PhantomData<T>,
}

impl<T> Clone for CallbackAccessor<T> {
    fn clone(&self) -> Self {
        Self {
            accessor_fns: self.accessor_fns.clone(),
            _phantom: self._phantom,
        }
    }
}

impl<T> CallbackAccessor<T>
where
    T: 'static,
{
    pub fn root() -> Self {
        Self {
            accessor_fns: vec![],
            _phantom: Default::default(),
        }
    }

    /// Returns a new CallbackAccessor that accesses a piece of the state of
    /// type U inside the current accessed value of type T
    pub fn drill_down<U>(
        &self,
        f: impl Fn(&mut T) -> &mut U + 'static + Clone,
    ) -> CallbackAccessor<U>
    where
        T: 'static,
        U: 'static,
    {
        let mut slicing_fns = self.accessor_fns.clone();

        let closure = ({
            fn funnel<Closure>(f: Closure) -> Closure
            where
                Closure: for<'a> Fn(&'a mut dyn Any) -> &'a mut dyn Any,
            {
                f
            }
            funnel::<_>
        })(move |t_any| f(t_any.downcast_mut().expect("Failed downcast")));

        slicing_fns.push(Box::new(closure));

        CallbackAccessor {
            accessor_fns: slicing_fns,
            _phantom: PhantomData::<U>::default(),
        }
    }

    pub fn access_any<'a>(&self, root: &'a mut dyn Any) -> &'a mut dyn Any {
        let mut curr = root;
        for f in &self.accessor_fns {
            curr = f.call(curr);
        }
        curr
    }

    pub fn callback<P>(&self, f: impl FnOnce(&mut T, P) + 'static) -> Callback<P> {
        let this: CallbackAccessor<T> = (*self).clone();
        let closure = move |root_any: &mut dyn Any, p: P| {
            let t: &mut T = this
                .access_any(root_any)
                .downcast_mut()
                .expect("Failed downcast");
            f(t, p);
        };
        Callback::External(ExternalCallback {
            input_type: TypeId::of::<T>(),
            f: Box::new(closure),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_drill_down() {
        #[derive(Default)]
        struct State {
            foo: Foo,
            bar: Bar,
        }
        #[derive(Default)]
        struct Foo {
            baz: Baz,
        }
        #[derive(Default)]
        struct Bar {
            x: f32,
        }
        #[derive(Default)]
        struct Baz {
            y: f32,
        }

        let state_cba = CallbackAccessor::<State>::root();

        let foo_cba = state_cba.drill_down(|state| &mut state.foo);
        let baz_cba = foo_cba.drill_down(|foo| &mut foo.baz);
        let bar_cba = state_cba.drill_down(|state| &mut state.bar);

        let mut test_state = State::default();

        let foo_dyn = foo_cba.access_any(&mut test_state);
        let _foo: &mut Foo = foo_dyn.downcast_mut().unwrap();

        let bar_dyn = bar_cba.access_any(&mut test_state);
        let bar: &mut Bar = bar_dyn.downcast_mut().unwrap();
        bar.x = 42.0;

        let baz_dyn = baz_cba.access_any(&mut test_state);
        let baz: &mut Baz = baz_dyn.downcast_mut().unwrap();
        baz.y = 123.4;

        assert_eq!(test_state.foo.baz.y, 123.4);
        assert_eq!(test_state.bar.x, 42.0);
    }
}
