use std::{
    cell::RefCell,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

use futures::task::{waker, ArcWake, AtomicWaker};
use parking_lot::Mutex;
use slotmap::{new_key_type, SlotMap};

use crate::Frame;

pub trait Effect<Data> {
    fn poll(self: Pin<&mut Self>, context: &mut Context, data: &mut Data) -> Poll<()>;
}

new_key_type! {struct TaskId; }

struct Task<Data> {
    effect: Pin<Box<dyn Effect<Data>>>,
    _marker: PhantomData<Data>,
}

impl<Data> Task<Data> {
    fn new(effect: Pin<Box<dyn Effect<Data>>>) -> Self {
        Self {
            effect,
            _marker: PhantomData,
        }
    }

    pub fn poll(&mut self, context: &mut Context, data: &mut Data) -> Poll<()> {
        self.effect.as_mut().poll(context, data)
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
    pub fn spawn(&self, effect: impl 'static + Effect<Data>) {
        let incoming = self.incoming.upgrade().expect("Executor dropped");
        let task = Task::new(Box::pin(effect));
        incoming.borrow_mut().push(task);
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
        {
            assert!(self.processing.is_empty());
            core::mem::swap(&mut *self.shared.ready.lock(), &mut self.processing);
        }

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

        for id in self.processing.drain(..) {
            let (task, waker) = self.tasks.get_mut(id).unwrap();
            let mut context = Context::from_waker(&*waker);
            tracing::debug!(?id, "Polling task");

            if task.poll(&mut context, data).is_ready() {
                tracing::debug!(?id, "Task completed");
                self.tasks.remove(id);
            }
        }
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

        spawner.spawn(FutureEffect::new(rx.into_recv_async(), |data, val| {
            *data = Some(val.unwrap());
        }));

        let mut data = None;

        ex.tick(&mut data);
        assert_eq!(data, None);

        tx.send(5).unwrap();

        ex.tick(&mut data);
        assert_eq!(data, Some(5));
    }
}
