use tween::Tweener;

use crate::{
    components::handle_detach,
    time::sleep,
    tweens::{ComponentTween, DynamicTween},
    Scope, Widget,
};

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
