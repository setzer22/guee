use epaint::ahash::{HashMap, HashSet};
use itertools::Itertools;
use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

/// A `PollToken` is returned when creating an internal callback. The same token
/// can then be reused to try fetch the result of the individual callback once
/// it runs.
///
/// The token is a cheaply copyable handle and can be freely shared or stored
/// around, but one should be careful to store them across frames because
/// callback data is removed from previous frames.
// #[derive(Copy, Clone)] <- see below
pub struct PollToken<T> {
    token: usize,
    _phantom: PhantomData<T>,
}

impl<P> PollToken<P> {
    pub fn as_raw(&self) -> RawPollToken {
        RawPollToken { token: self.token }
    }
}

/// Type-erased `PollToken`. Used by the internal implementation.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct RawPollToken {
    token: usize,
}

/// An external callback. See [`Callback`]
pub struct ExternalCallback<P> {
    pub input_type: TypeId,
    #[allow(clippy::type_complexity)]
    pub f: Box<dyn FnOnce(&mut dyn Any, P)>,
}

/// A type-erased callback function. Can be internal or external. Most users
/// will want to use external callbacks. Widget authors might want to use
/// internal callbacks to connect child widgets to their parents. See the docs
/// on each individual variant for more details.
pub enum Callback<P> {
    /// An external callback is provided by end-user, and its invocation is
    /// deferred until the end of each frame.
    ///
    /// This kind of callback consists of a function. That function will be
    /// called by guee, providing mutable access to a portion of the app state,
    /// plus the callback's payload, which is generally event data.
    External(ExternalCallback<P>),
    /// An internal callback is not exactly a callback. It is a mechanism used
    /// by widget authors, allowing listening for the events emitted by other
    /// widgets. It works via a polling mechanism: When a widget dispatches an
    /// event, and that event corresponds to an internal callback, the payload
    /// is stored internally so the parent widget who set up the callback can
    /// fetch it back via its corresponding [`PollToken`]
    Internal { token: PollToken<P> },
}

impl<P> Callback<P> {
    /// Constructs an external callback from the given function
    pub fn from_fn<F, T>(f: F) -> Callback<P>
    where
        T: 'static,
        F: FnOnce(&mut T, P) + 'static,
    {
        let closure = move |t_any: &mut dyn Any, p: P| {
            let t: &mut T = t_any.downcast_mut().expect("Failed downcast");
            f(t, p);
        };
        Callback::External(ExternalCallback {
            input_type: TypeId::of::<T>(),
            f: Box::new(closure),
        })
    }
}

pub struct StateAccessor {
    #[allow(unused)] // Will use them later
    input_type: TypeId,
    #[allow(unused)]
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

    pub fn invoke_callback(&self, state: &mut dyn Any, cd: DispatchedExternalCallback) {
        // Explicit deref necessary to differentiate between getting the type_id
        // of the inner type or the reference itself
        let state_type = (*state).type_id();
        if state_type == cd.input_type {
            cd.invoke(state);
        } else {
            let projected = self.access(state, state_type, cd.input_type);
            cd.invoke(projected);
        }
    }
}

/// A dispatched callback is a type-erased external callback (no generic P) plus
/// its type-erased payload and an invoker function that can be used to run the
/// actual callback code.
///
/// Callback dispatches are generated by widgets code, typically during
/// on_event, to enqueue some things to be called later, when the app state can
/// be accessed.
pub struct DispatchedExternalCallback {
    // The input type of the callback
    input_type: TypeId,
    // The type-erased external callback
    callback: Box<dyn Any>,
    // The stored payload to call the callback with
    payload: Box<dyn Any>,
    // The invoker is a function that takes an erased callback, an erased state
    // and an erased payload, downcasts everything and invokes the callback.
    #[allow(clippy::type_complexity)]
    invoker: Box<dyn FnOnce(Box<dyn Any>, &mut dyn Any, Box<dyn Any>)>,
}

impl DispatchedExternalCallback {
    pub fn new<P: 'static>(c: ExternalCallback<P>, payload: P) -> Self {
        let closure = |cb: Box<dyn Any>, input: &mut dyn Any, p: Box<dyn Any>| {
            let cb: ExternalCallback<P> = *cb.downcast().expect("Downcast failed");
            let p: P = *p.downcast().expect("Downcast failed");
            (cb.f)(input, p);
        };
        DispatchedExternalCallback {
            input_type: c.input_type,
            callback: Box::new(c),
            payload: Box::new(payload),
            invoker: Box::new(closure),
        }
    }

    pub fn invoke(self, state: &mut dyn Any) {
        (self.invoker)(self.callback, state, self.payload)
    }
}

#[derive(Default)]
pub struct DispatchedCallbackStorage {
    /// Stores the results of dispatched callbacks, to be invoked later on when
    /// there's mutable access to the state. Cleared at the end of the frame.
    pub external: Vec<DispatchedExternalCallback>,
    /// Maps poll tokens to the corresponding (type-erased) payload data
    /// returned by the function. Cleared at the end of the frame.
    pub internal: HashMap<RawPollToken, Box<dyn Any>>,
    /// The integer id for the next PollToken to be returned. Reset at the end
    /// of the frame.
    pub next_token: usize,
}

impl DispatchedCallbackStorage {
    pub fn dispatch_callback<P: 'static>(&mut self, c: Callback<P>, payload: P) {
        match c {
            Callback::External(ext) => self
                .external
                .push(DispatchedExternalCallback::new(ext, payload)),
            Callback::Internal { token } => {
                self.internal.insert(token.as_raw(), Box::new(payload));
            }
        }
    }

    /// Call at the end of the frame to run any pending external callbacks and
    /// clean up callback storage for the next frame.
    pub fn end_frame(&mut self, state: &mut dyn Any, accessor_registry: &AccessorRegistry) {
        self.internal.clear();
        for callback in self.external.drain(..) {
            accessor_registry.invoke_callback(state, callback);
        }
        self.next_token = 0;
    }

    /// Creates an internal callback, to be dispatched later via
    /// `dispatch_callback`. Returns both the callback object and the
    /// `PollToken` that calling code can use to fetch the result.
    pub fn create_internal_callback<P: 'static>(&mut self) -> (Callback<P>, PollToken<P>) {
        let token = PollToken::<P> {
            token: self.next_token,
            _phantom: Default::default(),
        };
        self.next_token += 1;
        (Callback::Internal { token }, token)
    }

    /// After an internal callback is fired (and before the end of the frame),
    /// call this function to obtain the callback result via its `PollToken`.
    ///
    /// Note that calling this function will remove the polled value from
    /// storage, and subsequent calls will return None.
    pub fn poll_callback_result<P: 'static>(&mut self, tk: PollToken<P>) -> Option<P> {
        self.internal
            .remove(&tk.as_raw())
            .map(|x| *x.downcast::<P>().expect("Failed downcast"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let mut storage = DispatchedCallbackStorage::default();

        storage.dispatch_callback(Callback::from_fn(|bar: &mut Bar, _| bar.x = 123.4), ());
        storage.dispatch_callback(Callback::from_fn(|baz: &mut Baz, _| baz.y = 432.1), ());
        storage.end_frame(&mut state, &registry);

        assert_eq!(state.bar.x, 123.4);
        assert_eq!(state.foo.baz.y, 432.1);
    }

    #[test]
    fn test_internal_callbacks() {
        let mut storage = DispatchedCallbackStorage::default();
        let (cb, tk) = storage.create_internal_callback();
        assert_eq!(storage.poll_callback_result(tk), None);
        storage.dispatch_callback(cb, "TestString".to_string());
        assert_eq!(storage.poll_callback_result(tk).unwrap(), "TestString");
    }
}

// Boilerplate: Rust doesn't allow derives with PhantomData

impl<P> Clone for PollToken<P> {
    fn clone(&self) -> Self {
        Self {
            token: self.token.clone(),
            _phantom: self._phantom.clone(),
        }
    }
}

impl<P> Copy for PollToken<P> {}
