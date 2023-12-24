use std::sync::{mpsc, Arc, Mutex};

enum Message<T> {
    Exit,
    Data(T),
}

pub trait Executor<T> {
    fn execute(&self, id: u32, data: T);
}

enum PoolState {
    Active,
    Inactive,
}

#[derive(Debug)]
pub enum WorkerPoolError {
    WorkersInactive,
}

pub struct WorkerPool<T, E>
where
    T: 'static,
    E: Executor<T> + 'static,
{
    state: PoolState,
    executor: E,
    sender: mpsc::Sender<Message<T>>,
    receiver: Arc<Mutex<mpsc::Receiver<Message<T>>>>,
    worker_count: u32,
    handlers: Vec<std::thread::JoinHandle<()>>,
}

impl<T, E> WorkerPool<T, E>
where
    T: 'static,
    T: Clone + Send + Sync,
    E: Executor<T> + 'static,
    E: Clone + Send + Sync,
{
    pub fn new(executor: E, worker_count: u32) -> Self {
        let (sender, receiver) = mpsc::channel();
        let receiver: Arc<Mutex<mpsc::Receiver<Message<T>>>> = Arc::new(Mutex::new(receiver));
        WorkerPool {
            state: PoolState::Inactive,
            executor: executor,
            sender,
            receiver,
            worker_count,
            handlers: vec![],
        }
    }

    pub fn start(&mut self) {
        for id in 0..self.worker_count {
            let receiver = self.receiver.clone();
            let executor = self.executor.clone();
            let worker_id = id.clone();
            let handler = std::thread::spawn(move || loop {
                let message = receiver.lock().unwrap().recv().unwrap();
                match message {
                    Message::Exit => {
                        println!("Worker {} exiting", worker_id);
                        break;
                    }
                    Message::Data(data) => {
                        executor.execute(worker_id, data);
                    }
                }
            });
            self.handlers.push(handler);
        }
        self.state = PoolState::Active;
    }

    pub fn stop(&mut self) {
        self.state = PoolState::Inactive;
        for _ in 0..self.worker_count {
            self.sender.send(Message::Exit).unwrap();
        }
        for handler in self.handlers.drain(..) {
            handler.join().unwrap();
        }
        self.handlers.clear();
    }

    pub fn execute(&self, data: T) -> Result<(), WorkerPoolError> {
        match self.state {
            PoolState::Inactive => return Err(WorkerPoolError::WorkersInactive),
            _ => {} // Do nothing if the pool is active. We'll get a SendError if the pool is inactive.
        }
        self.sender
            .send(Message::Data(data))
            .map_err(|_| WorkerPoolError::WorkersInactive)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert!(true);
    }
}
