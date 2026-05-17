

use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(Message::NewJob(job)) => {
                    // Execute the job
                    job();
                }
                Ok(Message::Terminate) => {
                    break;
                }
                Err(_) => {
                    // Channel closed, exit
                    break;
                }
            }
        });

        Self {
            id,
            thread: Some(thread),
        }
    }
}

/// Thread pool with fixed number of worker threads
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Message>>,
}

impl ThreadPool {
    /// Create a new thread pool with the specified number of threads
    ///
    /// # Panics
    /// Panics if size is 0
    #[must_use]
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "Thread pool size must be greater than 0");

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self {
            workers,
            sender: Some(sender),
        }
    }

    /// Execute a closure on the thread pool
    ///
    /// # Panics
    /// Panics if the thread pool has been shut down
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender
            .as_ref()
            .expect("Thread pool has been shut down")
            .send(Message::NewJob(job))
            .expect("Failed to send job to thread pool");
    }

    /// Get the number of worker threads
    #[must_use]
    pub fn size(&self) -> usize {
        self.workers.len()
    }

    /// Gracefully shutdown the thread pool
    pub fn shutdown(&mut self) {
        // Drop the sender to close the channel
        drop(self.sender.take());

        // Send terminate message to all workers
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().expect("Worker thread panicked");
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        if self.sender.is_some() {
            self.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn test_thread_pool_creation() {
        let pool = ThreadPool::new(4);
        assert_eq!(pool.size(), 4);
    }

    #[test]
    #[should_panic(expected = "Thread pool size must be greater than 0")]
    fn test_zero_size_panics() {
        let _pool = ThreadPool::new(0);
    }

    #[test]
    fn test_execute_job() {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicUsize::new(0));
        
        let counter_clone = Arc::clone(&counter);
        pool.execute(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Wait for job to complete
        thread::sleep(Duration::from_millis(100));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_multiple_jobs() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..10 {
            let counter_clone = Arc::clone(&counter);
            pool.execute(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(10));
            });
        }

        // Wait for all jobs to complete
        thread::sleep(Duration::from_millis(200));
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_graceful_shutdown() {
        let mut pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..5 {
            let counter_clone = Arc::clone(&counter);
            pool.execute(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(50));
            });
        }

        pool.shutdown();
        
        // After shutdown, all jobs should have completed
        let final_count = counter.load(Ordering::SeqCst);
        assert!(final_count > 0);
    }
}