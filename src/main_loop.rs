use async_io::Timer;
use async_task::{Runnable, Task};
use iced::futures::{Stream, StreamExt};
use std::future::Future;
use std::sync::LazyLock;
use std::time::Duration;

struct Queue {
    scheduled: async_channel::Sender<Runnable>,
    pending: async_channel::Receiver<Runnable>,
    wake: async_channel::Sender<()>,
    woken: async_channel::Receiver<()>,
}

static QUEUE: LazyLock<Queue> = LazyLock::new(|| {
    let (scheduled, pending) = async_channel::unbounded();
    let (wake, woken) = async_channel::bounded(1);
    Queue {
        scheduled,
        pending,
        wake,
        woken,
    }
});

pub struct Source(Option<Task<()>>);

impl Source {
    pub fn remove(mut self) {
        self.0.take();
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.detach();
        }
    }
}

pub fn spawn_local(future: impl Future<Output = ()> + 'static) -> Source {
    let (runnable, task) = async_task::spawn_local(future, schedule);
    runnable.schedule();
    Source(Some(task))
}

pub fn idle_once(work: impl FnOnce() + 'static) -> Source {
    spawn_local(async move { work() })
}

pub fn timeout_once(delay: Duration, work: impl FnOnce() + 'static) -> Source {
    spawn_local(async move {
        Timer::after(delay).await;
        work();
    })
}

pub fn timeout_repeating(interval: Duration, mut work: impl FnMut() -> Repeat + 'static) -> Source {
    spawn_local(async move {
        let mut timer = Timer::interval(interval);
        loop {
            (&mut timer).await;
            if work() == Repeat::Stop {
                break;
            }
        }
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    Continue,
    Stop,
}

pub fn run_pending() {
    let queue = &*QUEUE;
    for _ in 0..queue.pending.len() {
        match queue.pending.try_recv() {
            Ok(runnable) => {
                runnable.run();
            }
            Err(_) => break,
        }
    }
    if !queue.pending.is_empty() {
        let _ = queue.wake.try_send(());
    }
}

pub fn ticks(interval: &Duration) -> impl Stream<Item = ()> + use<> {
    Timer::interval(*interval).map(|_| ())
}

pub fn wakeups() -> impl Stream<Item = ()> {
    QUEUE.woken.clone()
}

fn schedule(runnable: Runnable) {
    let queue = &*QUEUE;
    let _ = queue.scheduled.try_send(runnable);
    let _ = queue.wake.try_send(());
}
