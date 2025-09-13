use std::{
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
};

use atomic_refcell::AtomicRef;
use flax::{
    component::ComponentValue,
    components::{child_of, name},
    entity_ids,
    error::MissingComponent,
    Component, Entity, EntityBuilder, EntityRef, EntityRefMut, Query, World,
};
use futures::{Future, Stream};
use pin_project::pin_project;
use tween::{Tween, TweenValue, Tweener};

use crate::{
    assets::AssetCache,
    atom::Atom,
    components::{children, context_store, handle_detach, handles},
    effect::Effect,
    executor::TaskHandle,
    input::InputEventHandler,
    stored::{UntypedHandle, WeakHandle},
    style::get_stylesheet_from_entity,
    systems::widget_template,
    tweens::{self, ComponentTween},
    Frame, FutureEffect, StreamEffect, Widget,
};

/// The scope to modify and mount a widget
pub struct Scope<'a> {
    frame: &'a mut Frame,
    id: Entity,
    data: EntityBuilder,
}

impl std::fmt::Debug for Scope<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl<'a> Scope<'a> {
    pub(crate) fn create(frame: &'a mut Frame, name: String) -> Self {
        let mut entity = EntityBuilder::new();
        widget_template(&mut entity, name);
        let id = entity.spawn(frame.world_mut());

        Self {
            frame,
            id,
            data: EntityBuilder::new(),
        }
    }

    pub fn new(entity: EntityRef<'a>, frame: &'a mut Frame) -> Self {
        Self {
            frame,
            id: entity.id(),
            data: EntityBuilder::new(),
        }
    }

    pub fn try_from_id(frame: &'a mut Frame, id: Entity) -> Option<Self> {
        if frame.world().is_alive(id) {
            Some(Self {
                frame,
                id,
                data: EntityBuilder::new(),
            })
        } else {
            None
        }
    }

    pub fn flush(&mut self) {
        let _ = self.data.append_to(self.frame.world_mut(), self.id);
    }

    /// Sets the component value
    pub fn set<T>(&mut self, component: Component<T>, value: T) -> &mut Self
    where
        T: ComponentValue,
    {
        self.data.set(component, value);
        self
    }

    pub fn update_dedup<T>(
        &mut self,
        component: Component<T>,
        value: T,
    ) -> Result<(), MissingComponent>
    where
        T: PartialEq + ComponentValue,
    {
        self.flush();
        self.entity_mut().update_dedup(component, value)
    }

    pub fn update<T, U>(
        &mut self,
        component: Component<T>,
        f: impl FnOnce(&mut T) -> U,
    ) -> Result<U, MissingComponent>
    where
        T: ComponentValue,
    {
        self.flush();
        self.entity_mut().update(component, f)
    }

    /// Sets the components default value
    pub fn set_default<T>(&mut self, component: Component<T>) -> &mut Self
    where
        T: ComponentValue + Default,
    {
        self.data.set(component, Default::default());
        self
    }

    /// Shorthand for:
    ///
    /// ```rust,ignore
    /// if let Some(val) = val {
    ///     scope.set(val)
    /// }
    /// ```
    pub fn set_opt<T>(&mut self, component: Component<T>, value: Option<T>) -> &mut Self
    where
        T: ComponentValue,
    {
        if let Some(value) = value {
            self.data.set(component, value);
        }
        self
    }

    pub fn entity(&self) -> EntityRef {
        // assert!(self.data.is_empty(), "EntityBuilder not flushed");
        self.frame.world().entity(self.id).unwrap()
    }

    pub fn entity_mut(&mut self) -> EntityRefMut {
        self.flush();
        self.frame.world_mut().entity_mut(self.id).unwrap()
    }

    pub fn add_tween<T: ComponentValue + TweenValue, A: 'static + Send + Sync + Tween<T>>(
        &mut self,
        component: Component<T>,
        tween: Tweener<T, f32, A>,
    ) {
        self.flush();
        self.entity_mut()
            .entry(tweens::tweens())
            .or_default()
            .add_tween(Box::new(ComponentTween::new(component, tween)));
    }

    pub fn add_dyn_tween(&mut self, tween: Box<dyn tweens::DynamicTween>) {
        self.flush();
        self.entity_mut()
            .entry(tweens::tweens())
            .or_default()
            .add_tween(tween);
    }

    pub fn stop_tweens<T: ComponentValue + TweenValue>(&mut self, component: Component<T>) {
        self.flush();
        self.entity_mut()
            .entry(tweens::tweens())
            .or_default()
            .stop_tweens(component);
    }

    /// Attaches a widget in a sub-scope.
    pub fn attach<W: Widget>(&mut self, widget: W) -> Entity {
        self.flush();
        let mut entity = EntityBuilder::new();
        widget_template(&mut entity, widget.name());
        let id = entity.spawn(self.frame.world_mut());

        self.frame
            .world_mut()
            .entry(self.id, children())
            .unwrap()
            .or_default()
            .push(id);

        self.flush();

        let id = {
            let mut s = Scope::try_from_id(self.frame, id).unwrap();

            s.set(child_of(self.id), ());
            s.set(name(), widget.name());
            s.flush();

            widget.mount(&mut s);
            s.id
        };

        assert!(self.frame.world().is_alive(self.id));

        id
    }

    pub fn attach_at<W: Widget>(&mut self, position: usize, widget: W) -> Entity {
        self.flush();
        let mut entity = EntityBuilder::new();
        widget_template(&mut entity, widget.name());
        let id = entity.spawn(self.frame.world_mut());

        self.frame
            .world_mut()
            .entry(self.id, children())
            .unwrap()
            .or_default()
            .insert(position, id);

        self.flush();

        let id = {
            let mut s = Scope::try_from_id(self.frame, id).unwrap();

            s.set(child_of(self.id), ());
            s.set(name(), widget.name());
            s.flush();

            widget.mount(&mut s);
            s.id
        };

        assert!(self.frame.world().is_alive(self.id));

        id
    }

    /// Detaches a child from the current scope
    pub fn detach(&mut self, id: Entity) {
        assert!(
            self.children().contains(&id),
            "Attempt to despawn a widget {id} that is not a child of the current scope {}",
            self.id
        );

        let entity = self.frame.world.entity(id).unwrap();
        let handle_detach = entity
            .get_mut(handle_detach())
            .ok()
            .and_then(|mut v| v.take());

        if let Some(handle_detach) = handle_detach {
            handle_detach(&mut Scope::try_from_id(&mut self.frame, id).unwrap());
        } else {
            self.entity_mut()
                .get_mut(children())
                .unwrap()
                .retain(|&x| x != id);

            self.frame.world.despawn_recursive(id, child_of).unwrap();
        }
    }

    pub fn detach_all(&mut self) {
        let children = self.children().clone();
        for child in children.iter() {
            self.detach(*child);
        }
    }

    pub fn parent(&mut self) -> Option<Scope<'_>> {
        let Some((parent, _)) = self.entity().relations(child_of).next() else {
            return None;
        };

        Some(Scope::try_from_id(self.frame, parent).unwrap())
    }

    pub fn children(&self) -> AtomicRef<'_, Vec<Entity>> {
        self.entity().get(children()).unwrap()
    }

    /// Spawns an effect scoped to the lifetime of this entity and scope
    pub fn spawn_effect(&self, effect: impl 'static + for<'x> Effect<Scope<'x>>) -> TaskHandle {
        self.frame.spawn(ScopedEffect {
            id: self.id,
            effect,
        })
    }

    pub fn spawn(&self, fut: impl 'static + Future) -> TaskHandle {
        self.spawn_effect(FutureEffect::new(fut, |_: &mut Scope<'_>, _| {}))
    }

    /// Spawns a scoped stream invoking the callback in with the widgets scope for each item
    pub fn spawn_stream<S: 'static + Stream>(
        &mut self,
        stream: S,
        func: impl 'static + FnMut(&mut Scope<'_>, S::Item),
    ) -> TaskHandle {
        self.spawn_effect(StreamEffect::new(stream, func))
    }

    pub fn spawn_future<F: 'static + Future>(
        &mut self,
        stream: F,
        func: impl 'static + FnOnce(&mut Scope<'_>, F::Output),
    ) -> TaskHandle {
        self.spawn_effect(FutureEffect::new(stream, func))
    }

    /// Spawns an effect which is *not* scoped to the widget
    pub fn spawn_unscoped(&self, effect: impl 'static + for<'x> Effect<Frame>) {
        self.frame.spawn(effect);
    }

    pub fn id(&self) -> Entity {
        self.id
    }

    pub fn assets_mut(&mut self) -> &mut AssetCache {
        &mut self.frame.assets
    }

    pub fn frame(&self) -> &Frame {
        self.frame
    }

    pub fn frame_mut(&mut self) -> &mut Frame {
        self.frame
    }

    pub fn world(&self) -> &World {
        &self.frame.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.frame.world
    }

    pub fn set_atom<T: ComponentValue>(&mut self, atom: Atom<T>, value: T) {
        self.frame.set_atom(atom, value);
    }

    /// Retrieves the value of an atom.
    ///
    /// Returns `None` if the atom does not exist.
    pub fn get_atom<T: ComponentValue>(&self, atom: Atom<T>) -> Option<AtomicRef<T>> {
        self.frame.get_atom(atom)
    }

    pub fn monitor_atom<T: ComponentValue>(
        &mut self,
        atom: Atom<T>,
        on_change: impl Fn(Option<&T>) + 'static,
    ) {
        self.frame.monitor_atom(atom, on_change);
    }

    /// Stores an arbitrary value and returns a handle to it.
    ///
    /// The value is stored for the duration of the widgets lifetime.
    ///
    /// A handle can be used to safely store state across multiple widgets and will not panic if
    /// the original widget is despawned.
    pub fn store<T: 'static>(&mut self, value: T) -> WeakHandle<T> {
        let handle = self.frame.store_mut().insert(value);
        let weak_handle = handle.downgrade();
        self.entity_mut()
            .entry(handles())
            .or_default()
            .push(UntypedHandle::new(handle));
        weak_handle
    }

    pub fn read<T: 'static>(&self, handle: &WeakHandle<T>) -> &T {
        let store = self.frame.store().store::<T>().expect("Handle is invalid");
        let handle = handle.upgrade(store).expect("Handle is invalid");
        self.frame.store().get(&handle)
    }

    pub fn write<T: 'static>(&mut self, handle: WeakHandle<T>) -> &mut T {
        let store = self.frame.store().store::<T>().expect("Handle is invalid");
        let handle = handle.upgrade(store).expect("Handle is invalid");
        self.frame.store_mut().get_mut(&handle)
    }

    pub fn monitor<T: ComponentValue>(
        &mut self,
        component: Component<T>,
        on_change: impl FnMut(Option<&T>) + 'static,
    ) {
        self.frame.monitor(self.id, component, on_change);
    }

    pub fn set_context<T: ComponentValue>(&mut self, context: Component<T>, value: T) {
        let mut query = Query::new(entity_ids()).with(context_store(self.id()));
        let store = query.borrow(self.frame.world()).first();

        if let Some(store) = store {
            self.frame.world.set(store, context, value).unwrap();
        } else {
            Entity::builder()
                .set(context_store(self.id), ())
                .set(context, value)
                .spawn(self.frame.world_mut());
        }
    }

    pub fn get_context<T: ComponentValue>(&self, context: Component<T>) -> AtomicRef<T> {
        match get_context(self.entity(), &self.frame.world, context) {
            Some(v) => v,
            None => {
                panic!("Missing context {context}");
            }
        }
    }

    pub fn get_context_cloned<T: ComponentValue + Clone>(&self, context: Component<T>) -> T {
        self.get_context(context).clone()
    }

    /// Invokes the provided callback when the targeted event is dispatched to the entity
    pub fn on_event<T: 'static>(
        &mut self,
        event: Component<InputEventHandler<T>>,
        func: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>, T) -> Option<T>,
    ) -> &mut Self {
        self.set(event, Box::new(func) as _)
    }

    /// Returns the active stylesheet for this scope
    pub fn stylesheet(&self) -> EntityRef {
        get_stylesheet_from_entity(&self.entity())
    }
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        self.flush()
    }
}

/// A non-mutable view into a widgets scope.
///
/// This is used for accessing state and modifying components (but not adding) of a widget during
/// callbacks.
pub struct ScopeRef<'a> {
    frame: &'a Frame,
    entity: EntityRef<'a>,
}

impl std::fmt::Debug for ScopeRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopeRef")
            .field("id", &self.entity.id())
            .finish_non_exhaustive()
    }
}

impl<'a> Deref for ScopeRef<'a> {
    type Target = EntityRef<'a>;

    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl<'a> ScopeRef<'a> {
    pub fn from_scope(scope: &'a Scope<'_>) -> Self {
        let entity = scope.entity();
        let frame = scope.frame();
        Self { frame, entity }
    }

    pub fn new(frame: &'a Frame, entity: EntityRef<'a>) -> Self {
        Self { frame, entity }
    }

    pub fn entity(&self) -> &EntityRef {
        &self.entity
    }

    /// Returns the active stylesheet for this scope
    pub fn stylesheet(&self) -> EntityRef {
        get_stylesheet_from_entity(self.entity())
    }

    /// Spawns an effect scoped to the lifetime of this entity and scope
    pub fn spawn_effect(&self, effect: impl 'static + for<'x> Effect<Scope<'x>>) -> TaskHandle {
        self.frame.spawn(ScopedEffect {
            id: self.entity.id(),
            effect,
        })
    }

    pub fn spawn(&self, fut: impl 'static + Future) -> TaskHandle {
        self.spawn_effect(FutureEffect::new(fut, |_: &mut Scope<'_>, _| {}))
    }

    /// Spawns a scoped stream invoking the callback in with the widgets scope for each item
    pub fn spawn_stream<S: 'static + Stream>(
        &mut self,
        stream: S,
        func: impl 'static + FnMut(&mut Scope<'_>, S::Item),
    ) -> TaskHandle {
        self.spawn_effect(StreamEffect::new(stream, func))
    }

    /// Spawns an effect which is *not* scoped to the widget
    pub fn spawn_unscoped(&self, effect: impl 'static + for<'x> Effect<Frame>) -> TaskHandle {
        self.frame.spawn(effect)
    }

    pub fn id(&self) -> Entity {
        self.entity.id()
    }

    pub fn frame(&self) -> &Frame {
        self.frame
    }

    /// Retrieves the value of an atom.
    ///
    /// Returns `None` if the atom does not exist.
    pub fn get_atom<T: ComponentValue>(&self, atom: Atom<T>) -> Option<AtomicRef<T>> {
        self.frame.get_atom(atom)
    }

    pub fn get_atom_cloned<T: ComponentValue + Clone>(&self, atom: Atom<T>) -> Option<T> {
        self.get_atom(atom).map(|v| v.clone())
    }

    pub fn parent(&self) -> Option<ScopeRef<'a>> {
        let Some((parent, _)) = self.entity.relations(child_of).next() else {
            return None;
        };

        let parent = self.frame.world().entity(parent).unwrap();
        Some(ScopeRef::new(self.frame, parent))
    }

    pub fn root(&self) -> Entity {
        let mut cur = *self.entity();
        loop {
            let Some((parent, _)) = cur.relations(child_of).next() else {
                return cur.id();
            };

            cur = self.frame.world.entity(parent).unwrap();
        }
    }

    pub fn read<T: 'static>(&self, handle: WeakHandle<T>) -> &T {
        let store = self.frame.store().store::<T>().expect("Handle is invalid");
        let handle = handle.upgrade(store).expect("Handle is invalid");
        self.frame.store().get(&handle)
    }

    pub fn get_context<T: ComponentValue>(&self, context: Component<T>) -> AtomicRef<T> {
        match get_context(*self.entity(), &self.frame.world, context) {
            Some(v) => v,
            None => {
                panic!("Missing context {context}");
            }
        }
    }

    pub fn get_context_cloned<T: ComponentValue + Clone>(&self, context: Component<T>) -> T {
        self.get_context(context).clone()
    }

    pub fn add_tween<T: ComponentValue + TweenValue, A: 'static + Send + Sync + Tween<T>>(
        &mut self,
        component: Component<T>,
        tween: Tweener<T, f32, A>,
    ) -> Result<(), MissingComponent> {
        self.entity()
            .get_mut(tweens::tweens())?
            .add_tween(Box::new(ComponentTween::new(component, tween)));

        Ok(())
    }
}

#[pin_project]
pub(crate) struct ScopedEffect<E> {
    pub(crate) id: Entity,
    #[pin]
    pub(crate) effect: E,
}

impl<E: for<'x> Effect<Scope<'x>>> Effect<Frame> for ScopedEffect<E> {
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>, frame: &mut Frame) -> Poll<()> {
        let p = self.project();

        if let Some(mut scope) = Scope::try_from_id(frame, *p.id) {
            p.effect.poll(context, &mut scope)
        } else {
            Poll::Ready(())
        }
    }

    fn label(&self) -> Option<&str> {
        self.effect.label()
    }
}

fn get_context<'a, T: ComponentValue>(
    mut cur: EntityRef<'a>,
    world: &'a World,
    component: Component<T>,
) -> Option<AtomicRef<'a, T>> {
    loop {
        if let Some(context_store) = Query::new(entity_ids())
            .with(context_store(cur.id()))
            .borrow(world)
            .first()
        {
            if let Ok(value) = world.get(context_store, component) {
                return Some(value);
            }
        }

        let (parent, _) = cur.relations(child_of).next()?;

        cur = world.entity(parent).unwrap();
    }
}
