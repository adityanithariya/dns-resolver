use crossbeam_channel::{Sender, TrySendError};
use std::{io, thread};

type Job = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug, PartialEq, Eq)]
pub enum SubmitError {
    /// Every worker is busy and the queue is already at capacity. The caller
    /// decides what "overload" means for them -- for a UDP server, that's
    /// usually "drop this packet", which is indistinguishable from ordinary
    /// packet loss to a client that already has to handle retries/timeouts.
    QueueFull,
    /// The pool has been shut down (all workers gone); nothing will ever
    /// consume submitted jobs again.
    PoolShutDown,
}

impl std::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmitError::QueueFull => write!(f, "UDP server worker queue is full"),
            SubmitError::PoolShutDown => write!(f, "Worker pool has been shut down"),
        }
    }
}

impl std::error::Error for SubmitError {}

// Helper constructors for std::io::Error
impl SubmitError {
    pub fn into_io_error(self) -> io::Error {
        match self {
            // ErrorKind::WouldBlock is great for QueueFull if you want non-blocking semantics
            SubmitError::QueueFull => io::Error::new(io::ErrorKind::WouldBlock, self),
            // ErrorKind::Other fits generic runtime errors like pool shutdown
            SubmitError::PoolShutDown => io::Error::new(io::ErrorKind::Other, self),
        }
    }
}

impl From<SubmitError> for io::Error {
    fn from(err: SubmitError) -> Self {
        err.into_io_error()
    }
}

/// A fixed number of worker threads pulling jobs off a single bounded queue.
/// This is the direct fix for unbounded thread-per-request: total concurrent
/// work is capped at `num_workers`, and the queue absorbs a small burst above
/// that before `try_submit` starts returning QueueFull instead of spawning
/// more threads to cope.
pub struct WorkerPool {
    sender: Sender<Job>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl WorkerPool {
    pub fn new(num_workers: usize, queue_capacity: usize) -> Self {
        let (sender, receiver) = crossbeam_channel::bounded::<Job>(queue_capacity);

        let workers = (0..num_workers)
            .map(|_| {
                let receiver = receiver.clone();
                thread::spawn(move || {
                    loop {
                        let job = receiver.recv();
                        match job {
                            Ok(job) => job(),
                            Err(_) => break, // sender side dropped: pool is shutting down
                        }
                    }
                })
            })
            .collect();

        WorkerPool { sender, workers }
    }

    /// Never blocks. Either the job is accepted (running now or queued
    /// behind at most `queue_capacity` others), or it's rejected immediately
    /// so the caller can decide how to handle overload rather than piling up
    /// unbounded work.
    pub fn try_submit(&self, job: Job) -> Result<(), SubmitError> {
        match self.sender.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(SubmitError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(SubmitError::PoolShutDown),
        }
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        // Dropping `sender` (implicitly, as a field) would happen anyway at
        // end of drop, but taking the workers here makes shutdown explicit:
        // once the last SyncSender is gone, every worker's `recv()` returns
        // Err and its loop exits, so joining them here won't hang.
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Barrier;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn caps_concurrent_execution_at_worker_count() {
        let pool = WorkerPool::new(2, 4);
        let running = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));

        for _ in 0..6 {
            let running = Arc::clone(&running);
            let max_seen = Arc::clone(&max_seen);
            pool.try_submit(Box::new(move || {
                let concurrent_now = running.fetch_add(1, Ordering::SeqCst) + 1;
                max_seen.fetch_max(concurrent_now, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(100));
                running.fetch_sub(1, Ordering::SeqCst);
            }))
            .unwrap();
        }

        // Give all 6 jobs time to have run (2 workers x ~3 batches x 100ms).
        thread::sleep(Duration::from_millis(500));

        let seen = max_seen.load(Ordering::SeqCst);
        assert!(seen >= 1, "expected at least one job to have run");
        assert!(
            seen <= 2,
            "expected at most 2 jobs running concurrently (the worker count), saw {}",
            seen
        );
    }

    #[test]
    fn rejects_once_workers_and_queue_are_both_full() {
        let pool = WorkerPool::new(1, 1);

        let started = Arc::new(Barrier::new(2));
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();

        {
            let started = Arc::clone(&started);
            pool.try_submit(Box::new(move || {
                started.wait(); // signal that the sole worker has picked this up
                let _ = release_rx.recv(); // then hold the worker busy until released
            }))
            .unwrap();
        }
        started.wait(); // don't race: wait until the worker actually dequeued job 1

        // Queue capacity is 1, and the worker is occupied: this one should be
        // accepted into the queue...
        assert_eq!(pool.try_submit(Box::new(|| {})), Ok(()));

        // ...but with the worker busy AND the queue's one slot full, a third
        // submission must be rejected rather than spawning another thread.
        assert_eq!(
            pool.try_submit(Box::new(|| {})),
            Err(SubmitError::QueueFull)
        );

        release_tx.send(()).unwrap(); // let job 1 finish so the pool can join cleanly on drop
    }
}
