mod fetch;

use crate::collab::RingRange;
use std::time::Duration;
use tokio::{sync::watch::Receiver, time};

pub struct Worker {
    range_updates: Receiver<Option<RingRange>>,
}

impl Worker {
    pub fn new(range_updates: Receiver<Option<RingRange>>) -> Self {
        Self { range_updates }
    }

    pub fn start(self) -> impl FnOnce() {
        let task = tokio::spawn(async move {
            let range_updates = self.range_updates;

            let mut ticker = time::interval(Duration::from_secs(10));

            tokio::spawn(async move {
                let mut range_updates = range_updates.clone();
                while range_updates.changed().await.is_ok() {
                    if let Some(range) = &*range_updates.borrow() {
                        println!("{:?}", range);
                    }
                }
            });

            loop {
                ticker.tick().await;
            }
        });

        let close_function = move || {
            task.abort();
            log::info!("Worker stopped");
        };

        log::info!("Worker started");

        close_function
    }
}
