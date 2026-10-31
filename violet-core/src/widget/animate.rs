use std::time::Duration;

use tween::Tweener;

use crate::{
    components::{handle_detach, opacity},
    style::transition_secs,
    time::sleep,
    tweens::DynamicTween,
    Scope, Widget,
};

/// Fades the widget being mounted in to `opacity`, and out before it is detached
pub fn fade_in_out(scope: &mut Scope<'_>, target: f32) {
    let duration = transition_secs(&scope.stylesheet());

    scope.set(opacity(), 0.0);
    scope.add_tween(opacity(), Tweener::quad_out(0.0, target, duration));

    scope.set(
        handle_detach(),
        Some(Box::new(move |scope: &mut Scope| {
            let current = scope.entity().get_copy(opacity()).unwrap_or(target);
            scope.stop_tweens(opacity());
            scope.add_tween(opacity(), Tweener::quad_out(current, 0.0, duration));
            scope.spawn_future(sleep(Duration::from_secs_f32(duration)), |scope, _| {
                let id = scope.id();
                if let Some(mut parent) = scope.parent() {
                    parent.detach(id);
                }
            });
        })),
    );
}

pub struct AnimateLifecycle<W> {
    pub inner: W,
    pub on_mount: Box<dyn DynamicTween>,
    pub on_unmount: Box<dyn DynamicTween>,
}

impl<W: Widget> AnimateLifecycle<W> {
    pub fn new(
        inner: W,
        on_mount: impl 'static + DynamicTween,
        on_unmount: impl 'static + DynamicTween,
    ) -> Self {
        Self {
            inner,
            on_mount: Box::new(on_mount),
            on_unmount: Box::new(on_unmount),
        }
    }
}

impl<W: Widget> Widget for AnimateLifecycle<W> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        scope.add_dyn_tween(self.on_mount);
        self.inner.mount(scope);

        scope.set(
            handle_detach(),
            Some(Box::new(|scope: &mut Scope| {
                let duration = self.on_unmount.duration();
                scope.add_dyn_tween(self.on_unmount);
                scope.spawn_future(sleep(duration), |scope, _| {
                    let id = scope.id();
                    if let Some(mut parent) = scope.parent() {
                        parent.detach(id);
                    }
                });
            })),
        );
    }
}
