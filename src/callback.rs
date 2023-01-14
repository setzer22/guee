use std::any::{Any, TypeId};

use epaint::ahash::{HashMap, HashSet};
use itertools::Itertools;

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

pub struct StateAccessor {
    input_type: TypeId,
    output_type: TypeId,
    #[allow(clippy::type_complexity)]
    accessor_fn: Box<dyn for<'a> Fn(&'a mut dyn Any) -> &'a mut dyn Any>,
}

impl StateAccessor {
    pub fn from_fn<F, T, U>(f: F) -> Self
    where
        F: Fn(&mut T) -> &mut U + 'static,
        T: 'static,
        U: 'static,
    {
        // NOTE: This is a trick to annotate the closure using a HRTB, since
        // that's currently not supported by rust.
        //
        // This uses a similar trick to the `higher_order_closure` crate, but we
        // don't want an external dependency just for this.
        let closure = ({
            fn funnel<Closure>(f: Closure) -> Closure
            where
                Closure: for<'a> Fn(&'a mut dyn Any) -> &'a mut dyn Any,
            {
                f
            }
            funnel::<_>
        })(move |t_any| f(t_any.downcast_mut().expect("Failed downcast")));

        StateAccessor {
            input_type: TypeId::of::<T>(),
            output_type: TypeId::of::<U>(),
            accessor_fn: Box::new(closure),
        }
    }
}

#[derive(Default)]
pub struct AccessorRegistry {
    accessors: HashMap<(TypeId, TypeId), StateAccessor>,
}

impl AccessorRegistry {
    pub fn register_accessor<F, T, U>(&mut self, f: F)
    where
        F: Fn(&mut T) -> &mut U + 'static,
        T: 'static,
        U: 'static,
    {
        let accessor = StateAccessor::from_fn(f);
        self.accessors
            .insert((TypeId::of::<T>(), TypeId::of::<U>()), accessor);
    }

    pub fn find_path(&self, from_typ: TypeId, to_typ: TypeId) -> Vec<TypeId> {
        fn recursive(
            this: &AccessorRegistry,
            current: TypeId,
            target: TypeId,
            visited: &mut HashSet<TypeId>,
        ) -> Option<Vec<TypeId>> {
            visited.insert(current);
            if current == target {
                Some(vec![current])
            } else {
                for (src, dst) in this.accessors.keys() {
                    if current == *src {
                        if visited.contains(dst) {
                            panic!("Should be a DAG. TODO: Better error reporting")
                        }
                        if let Some(mut result) = recursive(this, *dst, target, visited) {
                            result.push(current);
                            return Some(result);
                        }
                    }
                }
                None
            }
        }

        if let Some(mut found) = recursive(self, from_typ, to_typ, &mut Default::default()) {
            found.reverse();
            found
        } else {
            panic!("No registered accessor from {from_typ:?} to {to_typ:?}");
        }
    }

    pub fn access<'a>(
        &self,
        from: &'a mut dyn Any,
        from_typ: TypeId,
        to_typ: TypeId,
    ) -> &'a mut dyn Any {
        let path = self.find_path(from_typ, to_typ);
        let mut to = from;
        for (src, dst) in path.iter().tuple_windows() {
            let acc = &self.accessors[&(*src, *dst)];
            to = (acc.accessor_fn)(to);
        }
        to
    }

    pub fn invoke_callback(&self, state: &mut dyn Any, cb: Callback) {
        // Explicit deref necessary to differentiate between getting the type_id
        // of the inner type or the reference itself
        let state_type = (*state).type_id();
        if state_type == cb.input_type {
            cb.call(state);
        } else {
            let projected = self.access(state, state_type, cb.input_type);
            cb.call(projected);
        }
    }
}


#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};

    use crate::callback::Callback;

    use super::AccessorRegistry;

    #[test]
    fn test_accessor_regitry() {
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

        let mut registry = AccessorRegistry::default();
        registry.register_accessor(|state: &mut State| &mut state.foo);
        registry.register_accessor(|state: &mut State| &mut state.bar);
        registry.register_accessor(|state: &mut Foo| &mut state.baz);

        let mut state = State::default();

        let bar_dyn = registry.access(&mut state, TypeId::of::<State>(), TypeId::of::<Bar>());
        let Bar { ref mut x } = bar_dyn.downcast_mut().unwrap();
        *x = 42.0;

        let baz_dyn: &mut dyn Any =
            registry.access(&mut state, TypeId::of::<State>(), TypeId::of::<Baz>());
        let Baz { ref mut y } = baz_dyn.downcast_mut().unwrap();
        *y = 9.99;

        assert_eq!(state.bar.x, 42.0);
        assert_eq!(state.foo.baz.y, 9.99);

        let cb = Callback::from_fn(|bar: &mut Bar| { bar.x = 123.4 });
        registry.invoke_callback(&mut state, cb);

        let cb = Callback::from_fn(|baz: &mut Baz| { baz.y = 432.1 });
        registry.invoke_callback(&mut state, cb);

        assert_eq!(state.bar.x, 123.4);
        assert_eq!(state.foo.baz.y, 432.1);
    }
}
