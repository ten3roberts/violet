use std::{
    collections::BTreeSet,
    marker::PhantomPinned,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
    thread::{self, Thread},
    time::{Duration, Instant},
};

use futures::{
    task::{noop_waker, ArcWake},
    Future,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pin_project::{pin_project, pinned_drop};
use slotmap::new_key_type;
mod interval;

pub use interval::{interval, interval_at, Interval};

pub static GLOBAL_TIMER: Lazy<TimersHandle> = Lazy::new(Timers::start);

pub fn sleep_until(deadline: Instant) -> Sleep {
    Sleep::new(&GLOBAL_TIMER, deadline)
}

pub fn sleep(duration: Duration) -> Sleep {
    Sleep::new(&GLOBAL_TIMER, Instant::now() + duration)
}

struct TimerEntry {
    waker: Mutex<Waker>,
    finished: AtomicBool,
    _pinned: PhantomPinned,
}

new_key_type! {
    pub struct TimerKey;
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
struct Entry {
    deadline: Instant,
    timer: *const TimerEntry,
}

unsafe impl Send for Entry {}
unsafe impl Sync for Entry {}

struct ThreadWaker {
    thread_id: Thread,
}

impl ArcWake for ThreadWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.thread_id.unpark()
    }
}

struct Inner {
    /// Invoked when there is a new timer
    waker: Waker,
    heap: BTreeSet<Entry>,
}

impl Inner {
    pub fn register(&mut self, deadline: Instant, timer: *const TimerEntry) {
        self.heap.insert(Entry { deadline, timer });

        self.waker.wake_by_ref();
    }

    fn remove(&mut self, deadline: Instant, timer: *const TimerEntry) {
        self.heap.remove(&Entry { deadline, timer });
    }
}

#[derive(Clone)]
pub struct TimersHandle {
    inner: Arc<Mutex<Inner>>,
}

pub struct Timers {
    inner: Arc<Mutex<Inner>>,
}

impl Timers {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                heap: BTreeSet::new(),
                waker: noop_waker(),
            })),
        }
    }

    /// Advances the timers, returning the next deadline
    pub fn tick(&mut self, time: Instant) -> Option<Instant> {
        let mut shared = self.inner.lock();
        let shared = &mut *shared;

        while let Some(entry) = shared.heap.first() {
            // All deadlines before now have been handled
            if entry.deadline > time {
                return Some(entry.deadline);
            }

            let entry = shared.heap.pop_first().unwrap();
            // Fire and wake the timer
            // # Safety
            // Sleep removes the timer when dropped
            // Drop is guaranteed due to Sleep being pinned when registered
            let timer = unsafe { &*(entry.timer) };

            timer.finished.store(true, Ordering::Release);
            timer.waker.lock().wake_by_ref();
        }

        None
    }

    /// Starts executing the timers in the background
    pub fn start() -> TimersHandle {
        let timers = Timers::new();
        let handle = timers.handle();
        std::thread::spawn(move || timers.run_blocking());
        handle
    }

    pub fn run_blocking(mut self) {
        let waker = Arc::new(ThreadWaker {
            thread_id: thread::current(),
        });

        self.inner.lock().waker = futures::task::waker(waker);

        loop {
            let now = Instant::now();
            let next = self.tick(now);

            if let Some(next) = next {
                thread::park_timeout(next - now)
            } else {
                thread::park()
            }
        }
    }

    /// Acquire a handle used to spawn timers
    pub fn handle(&self) -> TimersHandle {
        TimersHandle {
            inner: self.inner.clone(),
        }
    }
}

#[pin_project(PinnedDrop)]
/// Sleep future
pub struct Sleep {
    shared: Arc<Mutex<Inner>>,
    timer: TimerEntry,
    deadline: Instant,
    registered: bool,
}

impl std::fmt::Debug for Sleep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sleep")
            .field("deadline", &self.deadline)
            .finish()
    }
}

impl Sleep {
    pub(crate) fn new(handle: &TimersHandle, deadline: Instant) -> Self {
        Self {
            shared: handle.inner.clone(),
            timer: TimerEntry {
                waker: Mutex::new(noop_waker()),
                finished: AtomicBool::new(false),
                _pinned: PhantomPinned,
            },
            deadline,
            registered: false,
        }
    }

    pub fn reset(self: Pin<&mut Self>, deadline: Instant) {
        let (timer, cur_deadline) = self.unregister();
        *cur_deadline = deadline;
        timer.finished.store(false, Ordering::SeqCst);
    }

    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    fn unregister(self: Pin<&mut Self>) -> (&mut TimerEntry, &mut Instant) {
        let p = self.project();
        // This removes any existing reference to the TimerEntry pointer
        let mut shared = p.shared.lock();
        shared.remove(*p.deadline, p.timer);
        *p.registered = false;
        (p.timer, p.deadline)
    }

    fn register(self: Pin<&mut Self>) {
        let p = self.project();
        p.shared.lock().register(*p.deadline, p.timer);
        *p.registered = true;
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self
            .timer
            .finished
            .compare_exchange(true, false, Ordering::Release, Ordering::Relaxed)
            .is_ok()
        {
            Poll::Ready(())
        } else if !self.registered {
            *self.timer.waker.lock() = cx.waker().clone();
            self.register();

            Poll::Pending
        } else {
            *self.timer.waker.lock() = cx.waker().clone();
            Poll::Pending
        }
    }
}

#[pinned_drop]
impl PinnedDrop for Sleep {
    fn drop(self: Pin<&mut Self>) {
        if self.registered {
            self.unregister();
        }
    }
}

impl Default for Timers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub(crate) fn assert_dur(found: Duration, expected: Duration, msg: &str) {
    assert!(
        (found.as_millis().abs_diff(expected.as_millis())) < 10,
        "Expected {found:?} to be close to {expected:?}\n{msg}",
    )
}

#[cfg(test)]
mod test {
    use std::{eprintln, time::Duration};

    use futures::{stream, FutureExt, StreamExt};

    use super::*;

    #[test]
    fn sleep() {
        let timers = Timers::new();

        let handle = timers.handle();

        thread::spawn(move || timers.run_blocking());

        let now = Instant::now();
        futures::executor::block_on(async move {
            Sleep::new(&handle, Instant::now() + Duration::from_millis(500)).await;

            eprintln!("Timer 1 finished");

            let now = Instant::now();
            Sleep::new(&handle, Instant::now() + Duration::from_millis(1000)).await;

            Sleep::new(&handle, now - Duration::from_millis(100)).await;

            eprintln!("Expired timer finished")
        });

        assert_dur(now.elapsed(), Duration::from_millis(500 + 1000), "seq");

        eprintln!("Done");
    }

    #[test]
    fn sleep_join() {
        let timers = Timers::new();

        let handle = timers.handle();

        thread::spawn(move || timers.run_blocking());

        let now = Instant::now();
        futures::executor::block_on(async move {
            let sleep_1 = Sleep::new(&handle, Instant::now() + Duration::from_millis(500));

            eprintln!("Timer 1 finished");

            let now = Instant::now();
            let sleep_2 = Sleep::new(&handle, Instant::now() + Duration::from_millis(1000));

            let sleep_3 = Sleep::new(&handle, now - Duration::from_millis(100));

            futures::join!(sleep_1, sleep_2, sleep_3);

            eprintln!("Expired timer finished")
        });

        assert_dur(now.elapsed(), Duration::from_millis(1000), "join");
        eprintln!("Done");
    }

    #[test]
    fn sleep_race() {
        let timers = Timers::new();

        let handle = timers.handle();

        thread::spawn(move || timers.run_blocking());

        let now = Instant::now();
        futures::executor::block_on(async move {
            {
                let mut sleep_1 =
                    Sleep::new(&handle, Instant::now() + Duration::from_millis(500)).fuse();

                eprintln!("Timer 1 finished");

                let mut sleep_2 =
                    Sleep::new(&handle, Instant::now() + Duration::from_millis(1000)).fuse();

                futures::select!(_ = sleep_1 => {}, _ = sleep_2 => {});
            }

            Sleep::new(&handle, Instant::now() + Duration::from_millis(1500)).await;

            let _never_polled = Sleep::new(&handle, Instant::now() + Duration::from_millis(2000));
            futures::pin_mut!(_never_polled);
        });

        assert_dur(now.elapsed(), Duration::from_millis(2000), "race");

        eprintln!("Done");
    }

    #[test]
    fn sleep_identical() {
        let timers = Timers::new();

        let handle = timers.handle();

        thread::spawn(move || timers.run_blocking());

        let now = Instant::now();
        futures::executor::block_on(async move {
            let deadline = now + Duration::from_millis(500);
            stream::iter(
                (0..100)
                    .map(|_| Sleep::new(&handle, deadline))
                    .collect::<Vec<_>>(),
            )
            .buffered(2048)
            .for_each(|_| async {
                // Sleep::new(&handle, Instant::now() + Duration::from_millis(100)).await;
            })
            .await;
        });

        assert_dur(now.elapsed(), Duration::from_millis(500), "seq");

        eprintln!("Done");
    }
}
