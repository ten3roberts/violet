use std::sync::Arc;

use crate::{declare_atom, style::SizeExt, widget::Stack, Scope, Widget};
use flax::{component, Entity};
use glam::Vec2;
use parking_lot::Mutex;
use slotmap::{SecondaryMap, SlotMap};

enum OverlayCommand {
    Open(OverlayId, bool, Box<dyn Send + FnOnce() -> Box<dyn Widget>>),
    Replace(OverlayId, Box<dyn Send + FnOnce() -> Box<dyn Widget>>),
    Close(OverlayId),
}

impl OverlayCommand {
    pub fn open<W: 'static + Widget>(
        id: OverlayId,
        ctor: impl 'static + Send + FnOnce() -> W,
        top: bool,
    ) -> Self {
        Self::Open(id, top, Box::new(|| Box::new(ctor())))
    }
}

/// A special type of widget that can open on top of everything else.
///
/// Used to implement windows and dialogs
pub trait Overlay: 'static + Send {
    fn create(self, scope: &mut Scope<'_>, token: OverlayHandle);
}

#[derive(Clone)]
pub struct OverlayHandle {
    id: OverlayId,
    commands: flume::Sender<OverlayCommand>,
}

impl OverlayHandle {
    pub fn new(id: OverlayId, commands: flume::Sender<OverlayCommand>) -> Self {
        Self { id, commands }
    }

    pub fn replace(&self, float: impl Overlay) {
        let token = self.clone();
        let _ = self.commands.send(OverlayCommand::Replace(
            self.id,
            Box::new(move || Box::new(OverlayWidgetImpl { float, token })),
        ));
    }

    pub fn close(&self) {
        let _ = self.commands.send(OverlayCommand::Close(self.id));
    }
}

slotmap::new_key_type! {
    pub struct OverlayId;
}

/// Allows managing floats remotely
#[derive(Clone)]
pub struct OverlayState {
    inner: Arc<OverlayStateInner>,
}

pub struct OverlayStateInner {
    floats: Mutex<SlotMap<OverlayId, ()>>,
    cmd_tx: flume::Sender<OverlayCommand>,
    rx: flume::Receiver<OverlayCommand>,
}

impl OverlayState {
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();
        Self {
            inner: Arc::new(OverlayStateInner {
                floats: Default::default(),
                cmd_tx: tx,
                rx,
            }),
        }
    }

    pub fn open(&self, float: impl Overlay) -> OverlayHandle {
        let id = self.inner.floats.lock().insert(());
        let token = OverlayHandle::new(id, self.inner.cmd_tx.clone());
        let token2 = token.clone();

        let _ = self.inner.cmd_tx.send(OverlayCommand::open(
            id,
            move || OverlayWidgetImpl { float, token },
            true,
        ));

        token2
    }

    pub fn close(&self, id: OverlayId) {
        let _ = self.inner.cmd_tx.send(OverlayCommand::Close(id));
    }
}

impl Default for OverlayState {
    fn default() -> Self {
        Self::new()
    }
}

component! {
    pub overlay_state: OverlayState,
}

/// Manages opening and closing of toplevel floats/windows in the UI
pub struct OverlayStack {
    overlays: SecondaryMap<OverlayId, Entity>,
    state: OverlayState,
}

impl OverlayStack {
    pub fn new() -> Self {
        Self {
            overlays: Default::default(),
            state: OverlayState::new(),
        }
    }

    pub fn from_state(state: OverlayState) -> Self {
        Self {
            overlays: Default::default(),
            state,
        }
    }

    pub fn with_float(&mut self, float: impl Overlay) -> &mut Self {
        self.state.open(float);
        self
    }

    pub fn state(&self) -> &OverlayState {
        &self.state
    }
}

impl Default for OverlayStack {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for OverlayStack {
    fn mount(mut self, scope: &mut Scope<'_>) {
        let rx = self.state.inner.rx.clone();
        scope.set_context(overlay_state(), self.state);

        scope.spawn_stream(rx.into_stream(), move |scope, item| {
            match item {
                OverlayCommand::Open(float_id, top, ctor) => {
                    let widget = ctor();
                    let entity = if top {
                        scope.attach(widget)
                    } else {
                        scope.attach_at(0, widget)
                    };

                    self.overlays.insert(float_id, entity);
                }
                OverlayCommand::Replace(float_id, ctor) => {
                    let Some(float_entity) = self.overlays.get_mut(float_id) else {
                        return;
                    };

                    let index = scope
                        .children()
                        .iter()
                        .position(|&v| v == *float_entity)
                        .expect("Float not a child");

                    scope.detach(*float_entity);
                    let entity = scope.attach_at(index, ctor());
                    *float_entity = entity;
                }
                OverlayCommand::Close(float_id) => {
                    let Some(&float_entity) = self.overlays.get(float_id) else {
                        return;
                    };

                    scope.detach(float_entity);
                }
            }

            scope.flush();
        });

        Stack::new(())
            // .with_horizontal_alignment(Align::Center)
            // .with_vertical_alignment(Align::Center)
            .with_maximize(Vec2::ONE)
            .mount(scope);
    }
}

struct OverlayWidgetImpl<S> {
    float: S,
    token: OverlayHandle,
}

impl<S: Overlay> Widget for OverlayWidgetImpl<S> {
    fn mount(self, scope: &mut Scope<'_>) {
        self.float.create(scope, self.token);
    }
}
