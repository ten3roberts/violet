use std::{
    collections::BTreeSet,
    marker::PhantomPinned,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Weak,
    },
    task::{Context, Poll, Waker},
    thread::{self, Thread},
    time::{Duration, Instant},
};

use futures::{
    task::{noop_waker, ArcWake, AtomicWaker},
    Future,
};
use once_cell::sync::Lazy;
use parking_lot::{Condvar, Mutex};
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
    waker: AtomicWaker,
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
    handle_count: AtomicUsize,
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

pub struct TimersHandle {
    inner: Arc<Mutex<Inner>>,
}

impl Clone for TimersHandle {
    fn clone(&self) -> Self {
        self.inner
            .lock()
            .handle_count
            .fetch_add(1, Ordering::Relaxed);

        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Drop for TimersHandle {
    fn drop(&mut self) {
        eprintln!("Dropping timers handle");
        let inner = self.inner.lock();
        let count = inner.handle_count.fetch_sub(1, Ordering::Relaxed);
        eprintln!("count: {}", count);
        if count == 1 {
            eprintln!("Waking timers");
            inner.waker.wake_by_ref();
        }
    }
}

pub struct Timers {
    inner: Arc<Mutex<Inner>>,
}

pub struct TimersFinished;

impl Timers {
    pub fn new() -> (Self, TimersHandle) {
        let inner = Arc::new(Mutex::new(Inner {
            heap: BTreeSet::new(),
            waker: noop_waker(),
            handle_count: AtomicUsize::new(1),
        }));

        (
            Self {
                inner: inner.clone(),
            },
            TimersHandle { inner },
        )
    }

    /// Advances the timers, returning the next deadline
    fn tick(&mut self, time: Instant, waker: Waker) -> Result<Option<Instant>, TimersFinished> {
        let mut shared = self.inner.lock();
        shared.waker = waker;
        let shared = &mut *shared;

        while let Some(entry) = shared.heap.first() {
            // All deadlines before now have been handled
            if entry.deadline > time {
                return Ok(Some(entry.deadline));
            }

            let entry = shared.heap.pop_first().unwrap();
            // Fire and wake the timer
            // # Safety
            // Sleep removes the timer when dropped
            // Drop is guaranteed due to Sleep being pinned when registered
            let timer = unsafe { &*(entry.timer) };

            // Wake the future waiting on the timer
            timer.finished.store(true, Ordering::SeqCst);
            timer.waker.wake();
        }

        if shared.handle_count.load(Ordering::SeqCst) == 0 {
            return Err(TimersFinished);
        }

        Ok(None)
    }

    /// Starts executing the timers in the background
    pub fn start() -> TimersHandle {
        let (timers, handle) = Timers::new();
        std::thread::spawn(move || timers.run_blocking());
        handle
    }

    pub fn run_blocking(mut self) {
        let waker = Arc::new(ThreadWaker {
            thread_id: thread::current(),
        });

        let waker = futures::task::waker(waker);

        loop {
            let now = Instant::now();
            let next = match self.tick(now, waker.clone()) {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("No remaining references to timers");
                    break;
                }
            };

            if let Some(next) = next {
                let dur = next - now;
                eprintln!("Parking for {:?}", dur);
                thread::park_timeout(dur)
            } else {
                eprintln!("Parking indefinitely waiting for new timers");
                thread::park();
                eprintln!("Wokend with new timers");
            }
        }
    }
}

#[pin_project(PinnedDrop)]
/// Sleep future
pub struct Sleep {
    shared: Arc<Mutex<Inner>>,
    timer: Box<TimerEntry>,
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
            timer: Box::new(TimerEntry {
                waker: AtomicWaker::new(),
                finished: AtomicBool::new(false),
                _pinned: PhantomPinned,
            }),
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

    /// Removes the timer entry from the timers queue.
    ///
    /// The TimerEntry is no longer aliased and is safe to modify.
    fn unregister(self: Pin<&mut Self>) -> (&mut TimerEntry, &mut Instant) {
        let p = self.project();
        // This removes any existing reference to the TimerEntry pointer
        let mut shared = p.shared.lock();
        shared.remove(*p.deadline, &**p.timer);
        *p.registered = false;
        (p.timer, p.deadline)
    }

    fn register_deadline(self: Pin<&mut Self>) {
        let p = self.project();
        p.shared.lock().register(*p.deadline, &**p.timer);
        *p.registered = true;
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self
            .timer
            .finished
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            Poll::Ready(())
        } else if !self.registered {
            self.timer.waker.register(cx.waker());
            self.register_deadline();

            Poll::Pending
        } else {
            self.timer.waker.register(cx.waker());
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

#[cfg(test)]
pub(crate) fn assert_dur(found: Duration, expected: Duration, msg: &str) {
    assert!(
        (found.as_millis().abs_diff(expected.as_millis())) < 10,
        "Expected {found:?} to be close to {expected:?}\n{msg}",
    )
}

#[cfg(test)]
fn setup_timers() -> (TimersHandle, thread::JoinHandle<()>) {
    let (timers, handle) = Timers::new();

    let thread = thread::Builder::new()
        .name("Timer".into())
        .spawn(move || timers.run_blocking())
        .unwrap();

    (handle, thread)
}

#[cfg(test)]
mod test {
    use std::{eprintln, time::Duration};

    use futures::{stream, FutureExt, StreamExt};

    use super::*;

    #[test]
    fn sleep() {
        let (handle, j) = setup_timers();
        let now = Instant::now();
        futures::executor::block_on(async move {
            Sleep::new(&handle, Instant::now() + Duration::from_millis(500)).await;

            eprintln!("Timer 1 finished");

            let now = Instant::now();
            Sleep::new(&handle, Instant::now() + Duration::from_millis(1000)).await;

            Sleep::new(&handle, now - Duration::from_millis(100)).await;

            eprintln!("Expired timer finished")
        });

        #[cfg(not(miri))]
        assert_dur(now.elapsed(), Duration::from_millis(500 + 1000), "seq");
        j.join().unwrap();
    }

    #[test]
    fn sleep_join() {
        let (handle, j) = setup_timers();

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

        #[cfg(not(miri))]
        assert_dur(now.elapsed(), Duration::from_millis(1000), "join");
        j.join().unwrap();
    }

    #[test]
    fn sleep_race() {
        let (handle, j) = setup_timers();

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

        #[cfg(not(miri))]
        assert_dur(now.elapsed(), Duration::from_millis(2000), "race");
        j.join().unwrap();
    }

    #[test]
    fn sleep_identical() {
        let (handle, j) = setup_timers();

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

        #[cfg(not(miri))]
        assert_dur(now.elapsed(), Duration::from_millis(500), "seq");
        j.join().unwrap();
    }

    #[test]
    fn sleep_reset() {
        let (handle, j) = setup_timers();

        let now = Instant::now();
        futures::executor::block_on(async move {
            let sleep = Sleep::new(&handle, Instant::now() + Duration::from_millis(500));

            futures::pin_mut!(sleep);
            sleep.as_mut().await;

            sleep
                .as_mut()
                .reset(Instant::now() + Duration::from_millis(1000));

            sleep.as_mut().await;
        });

        #[cfg(not(miri))]
        assert_dur(now.elapsed(), Duration::from_millis(1500), "seq");
        j.join().unwrap();
    }
}
