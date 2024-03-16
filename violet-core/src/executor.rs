use std::{
    cell::RefCell,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

use futures::task::{waker, ArcWake, AtomicWaker};
use parking_lot::Mutex;
use slotmap::{new_key_type, SlotMap};

use crate::{effect::Effect, Frame};

new_key_type! {struct TaskId; }

const STATE_PENDING: u32 = 0;
const STATE_ABORTED: u32 = 1;
const STATE_FINISHED: u32 = 2;

struct TaskState {
    state: AtomicU32,
}

pub struct TaskHandle {
    join_state: Arc<TaskState>,
}

impl TaskHandle {
    pub fn abort(&self) {
        self.join_state.state.store(STATE_ABORTED, Ordering::SeqCst);
    }
}

struct Task<Data> {
    effect: Pin<Box<dyn Effect<Data>>>,
    join_state: Arc<TaskState>,
    _marker: PhantomData<Data>,
}

impl<Data> Task<Data> {
    fn new(effect: Pin<Box<dyn Effect<Data>>>) -> (Self, TaskHandle) {
        let state = Arc::new(TaskState {
            state: AtomicU32::new(STATE_PENDING),
        });

        let handle = TaskHandle {
            join_state: state.clone(),
        };

        (
            Self {
                effect,
                _marker: PhantomData,
                join_state: state.clone(),
            },
            handle,
        )
    }

    pub fn poll(&mut self, context: &mut Context, data: &mut Data) -> Poll<()> {
        let state = self.join_state.state.load(Ordering::Acquire);

        if state == STATE_ABORTED {
            return Poll::Ready(());
        }

        if self.effect.as_mut().poll(context, data).is_ready() {
            self.join_state
                .state
                .store(STATE_FINISHED, Ordering::Release);

            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

struct TaskWaker {
    id: TaskId,
    shared: Arc<Shared>,
}

impl ArcWake for TaskWaker {
    fn wake_by_ref(t: &Arc<Self>) {
        t.shared.push_ready(t.id);
    }
}

/// Is Send + Sync
struct Shared {
    woken: AtomicBool,
    executor_waker: AtomicWaker,
    ready: Mutex<Vec<TaskId>>,
}

impl Shared {
    fn push_ready(&self, id: TaskId) {
        self.ready.lock().push(id);
        self.woken.store(true, Ordering::Relaxed);
        self.executor_waker.wake();
    }
}

/// Allows executing futures
pub struct Executor<Data = Frame> {
    tasks: SlotMap<TaskId, (Task<Data>, Waker)>,
    processing: Vec<TaskId>,

    shared: Arc<Shared>,
    /// New tasks
    incoming: Rc<RefCell<Vec<Task<Data>>>>,
}

pub struct Spawner<Data> {
    incoming: std::rc::Weak<RefCell<Vec<Task<Data>>>>,
}

impl<Data> Spawner<Data> {
    pub fn spawn(&self, effect: impl 'static + Effect<Data>) -> TaskHandle {
        let incoming = self.incoming.upgrade().expect("Executor dropped");
        let (task, handle) = Task::new(Box::pin(effect));
        incoming.borrow_mut().push(task);

        handle
    }
}

impl<Data> Executor<Data> {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            executor_waker: AtomicWaker::new(),
            ready: Default::default(),
            woken: AtomicBool::new(false),
        });

        let incoming = Default::default();

        Self {
            tasks: SlotMap::with_key(),
            shared,
            processing: Vec::new(),
            incoming,
        }
    }

    /// Returns a thread local spawner
    pub fn spawner(&self) -> Spawner<Data> {
        Spawner {
            incoming: Rc::downgrade(&self.incoming),
        }
    }

    pub fn poll_tick(&mut self, data: &mut Data, cx: &mut Context<'_>) -> Poll<()> {
        self.shared.executor_waker.register(cx.waker());

        if self
            .shared
            .woken
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            tracing::info!("Executor ready");
            self.tick(data);
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub fn tick(&mut self, data: &mut Data) {
        puffin::profile_function!();
        assert!(self.processing.is_empty());
        loop {
            puffin::profile_scope!("tick");
            // Add new tasks
            self.processing
                .extend(self.incoming.borrow_mut().drain(..).map(|task| {
                    self.tasks.insert_with_key(|id| {
                        let waker = waker(Arc::new(TaskWaker {
                            id,
                            shared: self.shared.clone(),
                        }));

                        (task, waker)
                    })
                }));

            self.processing.append(&mut self.shared.ready.lock());

            if self.processing.is_empty() {
                break;
            }

            for id in self.processing.drain(..) {
                let Some((task, waker)) = self.tasks.get_mut(id) else {
                    // Task was canceled and thus returned Poll::Ready before being woken by an
                    // external waker
                    continue;
                };

                puffin::profile_scope!("process task", task.effect.label().unwrap_or_default());
                let mut context = Context::from_waker(&*waker);
                tracing::trace!(?id, "Polling task");

                if task.poll(&mut context, data).is_ready() {
                    self.tasks.remove(id);
                }
            }
        }
    }
}

impl<Data> Default for Executor<Data> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use crate::effect::FutureEffect;

    use super::*;

    #[test]
    fn single_test() {
        let (tx, rx) = flume::unbounded();

        let mut ex = Executor::new();

        let spawner = ex.spawner();

        spawner.spawn(FutureEffect::new(
            rx.into_recv_async(),
            |data: &mut Option<i32>, val: Result<i32, flume::RecvError>| {
                *data = Some(val.unwrap());
            },
        ));

        let mut data = None;

        ex.tick(&mut data);
        assert_eq!(data, None);

        tx.send(5).unwrap();

        ex.tick(&mut data);
        assert_eq!(data, Some(5));
    }
}
