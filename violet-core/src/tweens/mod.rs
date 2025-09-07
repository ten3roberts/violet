use crate::components::{app_instance, delta_time};
use flax::{
    component::{ComponentDesc, ComponentValue},
    system, Component, EntityRef, FetchExt,
};
use std::time::Duration;
pub use tween;
use tween::{Tween, TweenValue, Tweener};

pub struct Tweens {
    active_tweens: Vec<Box<dyn DynamicTween>>,
}

impl Tweens {
    pub fn new() -> Self {
        Self {
            active_tweens: Vec::new(),
        }
    }

    pub fn add_tween(&mut self, tween: Box<dyn DynamicTween>) {
        self.active_tweens.push(tween);
    }

    pub fn stop_tweens<T: ComponentValue>(&mut self, target: Component<T>) {
        self.active_tweens
            .retain_mut(|v| v.target() != target.desc());
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

pub trait DynamicTween: Send + Sync {
    fn update(&mut self, entity: EntityRef, delta: Duration) -> bool;
    fn target(&self) -> ComponentDesc;
    fn duration(&self) -> Duration;
}

impl<T, A> DynamicTween for ComponentTween<T, A>
where
    T: ComponentValue + TweenValue,
    A: Send + Sync + Tween<T>,
{
    fn update(&mut self, entity: EntityRef, delta: Duration) -> bool {
        let new_value = self.tween.move_by(delta.as_secs_f32());

        if let Ok(mut value) = entity.get_mut(self.target) {
            *value = new_value
        } else {
            tracing::error!(
                "Missing target component {:?} for tween on entity {}",
                self.target,
                entity,
            );
        }

        !self.tween.is_finished()
    }

    fn target(&self) -> ComponentDesc {
        self.target.desc()
    }

    fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.tween.duration)
    }
}

flax::component! {
    pub tweens: Tweens,
}

#[system(args(dt = delta_time().source(app_instance()).copied()))]
pub fn update_tweens_system(tweens: &mut Tweens, entity: EntityRef, dt: Duration) {
    tweens.update(entity, dt);
}
