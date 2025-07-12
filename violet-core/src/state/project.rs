use std::{marker::PhantomData, sync::Arc};

use futures::{stream::BoxStream, Stream, StreamExt};

use crate::state::{
    State, StateOwned, StateRef, StateSink, StateStream, StateStreamRef, StateWrite,
};

type DynProjectMutFunc<C, U> = Box<dyn Send + Sync + Fn(&mut <C as State>::Item) -> &mut U>;

type DynProjectFunc<C, U> = Box<dyn Send + Sync + Fn(&<C as State>::Item) -> &U>;

/// Transform state of one type to another using reference projection.
///
/// This is for example useful to create a substate targeting a field of a type.
///
/// **NOTE**: Requires that the underlying state supports [`StateMut`] and `[StateStreamRef]`.
///
///
/// # Notable Implementations
///
/// - [`StateMut`]: Allows directly writing into the substate.
/// - [`StateStreamRef`]: Allows receiving references to the substate.
/// - [`StateStream`] (if U: Clone): Allows receiving clones of the projected substate.
/// - [`StateSink`]: Allow sending a type U which will end up writing into the projection.
pub struct Project<
    C: State,
    U: ?Sized,
    F: Fn(&C::Item) -> &U = DynProjectFunc<C, U>,
    G: Fn(&mut C::Item) -> &mut U = DynProjectMutFunc<C, U>,
> {
    inner: C,
    project: Arc<(F, G)>,
    _marker: PhantomData<U>,
}

impl<C: State, U: ?Sized, F: Fn(&C::Item) -> &U, G: Fn(&mut C::Item) -> &mut U>
    Project<C, U, F, G>
{
    /// Creates a new state projection
    pub fn new(inner: C, project: F, project_mut: G) -> Self {
        Self {
            inner,
            project: Arc::new((project, project_mut)),
            _marker: PhantomData,
        }
    }
}

impl<C, U, F, G> Clone for Project<C, U, F, G>
where
    C: Clone + State,
    U: ?Sized,
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            project: self.project.clone(),
            _marker: PhantomData,
        }
    }
}

impl<C: State, U: ?Sized> Project<C, U> {
    pub fn new_dyn(
        inner: C,
        project: impl 'static + Send + Sync + Fn(&C::Item) -> &U,
        project_mut: impl 'static + Send + Sync + Fn(&mut C::Item) -> &mut U,
    ) -> Self {
        Self {
            inner,
            project: Arc::new((Box::new(project), Box::new(project_mut))),
            _marker: PhantomData,
        }
    }
}

impl<C: State, U: ?Sized, F, G> Project<C, U, F, G>
where
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    /// Changes projection from `T -> U` given a `U -> V` directly to a projection of `T -> V`.
    pub fn flat_project<V: ?Sized>(
        self,
        project: impl 'static + Send + Sync + Fn(&U) -> &V,
        project_mut: impl 'static + Send + Sync + Fn(&mut U) -> &mut V,
    ) -> Project<C, V>
    where
        F: 'static + Send + Sync + Fn(&C::Item) -> &U,
        G: 'static + Send + Sync + Fn(&mut C::Item) -> &mut U,
        U: 'static,
    {
        let project1 = self.project.clone();

        Project {
            inner: self.inner,
            project: Arc::new((
                Box::new(move |v| project((self.project.0)(v))),
                Box::new(move |v| project_mut((project1.1)(v))),
            )),
            _marker: PhantomData,
        }
    }
}

impl<C: State, U: ?Sized, F, G> State for Project<C, U, F, G>
where
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    type Item = U;
}

impl<C, U: ?Sized, F, G> StateRef for Project<C, U, F, G>
where
    C: StateRef,
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    fn read_ref<H: FnOnce(&Self::Item) -> V, V>(&self, f: H) -> V {
        self.inner.read_ref(|v| f((self.project.0)(v)))
    }
}

impl<C, U: ?Sized, F: Fn(&C::Item) -> &U, G: Fn(&mut C::Item) -> &mut U> StateOwned
    for Project<C, U, F, G>
where
    C: StateRef,
    U: Clone,
    F: Fn(&C::Item) -> &U,
{
    fn read(&self) -> Self::Item {
        self.read_ref(|v| v.clone())
    }
}

impl<C, U: ?Sized, F, G> StateWrite for Project<C, U, F, G>
where
    C: StateWrite,
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    fn write_mut<H: FnOnce(&mut Self::Item) -> V, V>(&self, f: H) -> V {
        self.inner.write_mut(|v| f((self.project.1)(v)))
    }
}

impl<C, U: ?Sized, F, G> StateStreamRef for Project<C, U, F, G>
where
    C: StateStreamRef,
    F: 'static + Send + Sync + Fn(&C::Item) -> &U,
    G: 'static + Send + Sync + Fn(&mut C::Item) -> &mut U,
{
    fn stream_ref<I, V>(&self, mut func: I) -> impl Stream<Item = V> + 'static + Send
    where
        I: 'static + Send + Sync + FnMut(&Self::Item) -> V,
        V: 'static + Send,
    {
        let project = self.project.clone();
        self.inner.stream_ref(move |v| func(project.0(v)))
    }
}

/// Owned state stream
impl<C, U: ?Sized, F, G> StateStream for Project<C, U, F, G>
where
    C: StateStreamRef,
    U: 'static + Send + Sync + Clone,
    F: 'static + Send + Sync + Fn(&C::Item) -> &U,
    G: 'static + Send + Sync + Fn(&mut C::Item) -> &mut U,
{
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        self.stream_ref(|v| v.clone()).boxed()
    }
}

/// Bridge update-by-reference to update-by-value
impl<C, U, F, G> StateSink for Project<C, U, F, G>
where
    C: StateWrite,
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    fn send(&self, value: Self::Item) {
        self.write_mut(|v| *v = value);
    }
}
