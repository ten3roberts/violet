use crate::components::{app_instance, delta_time};
use flax::{component, component::ComponentValue, system, Component, EntityRef, FetchExt};
use std::time::Duration;
use tween::{Tween, TweenValue, Tweener};

pub struct Tweens {
    active_tweens: Vec<Box<dyn ComponentTweenDyn>>,
}

impl Tweens {
    pub fn new() -> Self {
        Self {
            active_tweens: Vec::new(),
        }
    }

    pub fn add_tween<T, A>(&mut self, target: Component<T>, tween: Tweener<T, f32, A>)
    where
        T: ComponentValue + TweenValue,
        A: 'static + Send + Sync + Tween<T>,
    {
        self.active_tweens
            .push(Box::new(ComponentTween::new(target, tween)));
    }

    /// Update all active tweens
    pub fn update(&mut self, entity: EntityRef, delta: Duration) {
        self.active_tweens.retain_mut(|v| v.update(entity, delta));
    }
}

impl Default for Tweens {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ComponentTween<T, A> {
    target: Component<T>,
    tween: Tweener<T, f32, A>,
}

impl<T, A> ComponentTween<T, A> {
    pub fn new(target: Component<T>, tween: Tweener<T, f32, A>) -> Self {
        Self { target, tween }
    }
}

pub trait ComponentTweenDyn: Send + Sync {
    fn update(&mut self, entity: EntityRef, delta: Duration) -> bool;
}

impl<T, A> ComponentTweenDyn for ComponentTween<T, A>
where
    T: ComponentValue + TweenValue,
    A: Send + Sync + Tween<T>,
{
    fn update(&mut self, entity: EntityRef, delta: Duration) -> bool {
        let new_value = self.tween.move_by(delta.as_secs_f32());

        if let Ok(mut value) = entity.get_mut(self.target) {
            *value = new_value
        }

        !self.tween.is_finished()
    }
}

component! {
    pub tweens: Tweens,
}

#[system(args(dt = delta_time().source(app_instance()).copied()))]
pub fn update_tweens_system(tweens: &mut Tweens, entity: EntityRef, dt: Duration) {
    tweens.update(entity, dt);
}
