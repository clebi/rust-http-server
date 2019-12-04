use std::sync::{mpsc, Arc, Mutex};
use std::thread;

trait FnBox {
  fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
  fn call_box(self: Box<F>) {
    (*self)()
  }
}

/// Messages that can be received by a worker
enum Message {
  /// Job to run
  NewJob(Job),
  /// Terminate the worker
  Terminate,
}

/// The worker is responsible to run jobs into a thread
struct Worker {
  id: usize,
  thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
  /// Create a new worker
  ///
  /// # Arguments
  ///
  /// * `id` unique identifier
  /// * `receiver` is the channel used to receive jobs to run
  ///
  /// # Panics
  ///
  /// The `new` function will panic if it can't acquire the lock on the receiver
  /// The `new` function will panic if there is a problem with the channel
  pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
    let thread = thread::spawn(move || loop {
      let message = receiver.lock().unwrap().recv().unwrap();

      match message {
        Message::NewJob(job) => {
          println!("Worker {} got a job; executing.", id);
          job.call_box();
        }
        Message::Terminate => {
          println!("Worker {} was told to terminate", id);
          break;
        }
      }
    });

    Worker {
      id,
      thread: Some(thread),
    }
  }
}

type Job = Box<dyn FnBox + Send + 'static>;

/// ThreadPool manage a pool of threads
pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: mpsc::Sender<Message>,
}

impl ThreadPool {
  /// Create a new ThreadPool.
  ///
  /// # Arguments
  ///
  /// * `size` is the number of threads in the pool.
  ///
  /// # Returns
  ///
  /// The created thread pool
  ///
  /// # Panics
  ///
  /// The `new` function will panic if the size is zero
  pub fn new(size: usize) -> ThreadPool {
    assert!(size > 0);

    let (sender, receiver) = mpsc::channel();

    let receiver = Arc::new(Mutex::new(receiver));

    let mut workers = Vec::with_capacity(size);

    for id in 0..size {
      workers.push(Worker::new(id, Arc::clone(&receiver)));
    }

    ThreadPool { workers, sender }
  }

  /// Execute the function into a thread from the pool
  ///
  /// # Arguments
  ///
  /// * `f` is the function to execute
  ///
  /// # Panics
  ///
  /// The `execute` function will panic if it is unable to send the job to a thread
  pub fn execute<F>(&self, f: F)
  where
    F: FnOnce() + Send + 'static,
  {
    let job = Box::new(f);

    self.sender.send(Message::NewJob(job)).unwrap();
  }
}

impl Drop for ThreadPool {
  /// Drop the thread pool by terminating all its workers
  fn drop(&mut self) {
    println!("Sending terminate message to all workers.");
    for _ in &mut self.workers {
      self.sender.send(Message::Terminate).unwrap();
    }

    println!("Shutting down all workers.");

    for worker in &mut self.workers {
      println!("Shutting down worker {}", worker.id);

      if let Some(thread) = worker.thread.take() {
        thread.join().unwrap();
      }
    }
  }
}
