use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

type TaskFn = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct ScheduledTask {
    pub id: u64,
    pub execute_at_tick: u64,
    pub repeat_interval: Option<u64>,
    pub task: TaskFn,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        other.execute_at_tick.cmp(&self.execute_at_tick) // Min-heap
    }
}

pub struct TaskHandle {
    pub id: u64,
}

pub struct Scheduler {
    tasks: Mutex<BinaryHeap<ScheduledTask>>,
    current_tick: Mutex<u64>,
    next_id: Mutex<u64>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(BinaryHeap::new()),
            current_tick: Mutex::new(0),
            next_id: Mutex::new(1),
        }
    }

    pub fn schedule_delayed(&self, delay_ticks: u64, task: impl Fn() + Send + Sync + 'static) -> TaskHandle {
        let id = {
            let mut next = self.next_id.lock().unwrap();
            let id = *next;
            *next += 1;
            id
        };
        let current = *self.current_tick.lock().unwrap();
        let scheduled = ScheduledTask {
            id,
            execute_at_tick: current + delay_ticks,
            repeat_interval: None,
            task: Arc::new(task),
        };
        self.tasks.lock().unwrap().push(scheduled);
        TaskHandle { id }
    }

    pub fn schedule_repeating(
        &self,
        delay_ticks: u64,
        interval_ticks: u64,
        task: impl Fn() + Send + Sync + 'static,
    ) -> TaskHandle {
        let id = {
            let mut next = self.next_id.lock().unwrap();
            let id = *next;
            *next += 1;
            id
        };
        let current = *self.current_tick.lock().unwrap();
        let scheduled = ScheduledTask {
            id,
            execute_at_tick: current + delay_ticks,
            repeat_interval: Some(interval_ticks),
            task: Arc::new(task),
        };
        self.tasks.lock().unwrap().push(scheduled);
        TaskHandle { id }
    }

    pub fn cancel(&self, handle: &TaskHandle) {
        let mut tasks = self.tasks.lock().unwrap();
        let remaining: Vec<_> = tasks.drain().filter(|t| t.id != handle.id).collect();
        for task in remaining {
            tasks.push(task);
        }
    }

    pub async fn tick(&self) {
        let tick = {
            let mut current = self.current_tick.lock().unwrap();
            *current += 1;
            *current
        };

        let mut to_reschedule = Vec::new();

        loop {
            let task = {
                let mut tasks = self.tasks.lock().unwrap();
                if tasks.peek().is_some_and(|t| t.execute_at_tick <= tick) {
                    tasks.pop()
                } else {
                    None
                }
            };

            match task {
                Some(scheduled) => {
                    (scheduled.task)();
                    if let Some(interval) = scheduled.repeat_interval {
                        to_reschedule.push(ScheduledTask {
                            id: scheduled.id,
                            execute_at_tick: tick + interval,
                            repeat_interval: scheduled.repeat_interval,
                            task: scheduled.task,
                        });
                    }
                }
                None => break,
            }
        }

        let mut tasks = self.tasks.lock().unwrap();
        for task in to_reschedule {
            tasks.push(task);
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

    #[tokio::test]
    async fn test_delayed_task() {
        let scheduler = Scheduler::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        scheduler.schedule_delayed(2, move || {
            counter_clone.fetch_add(1, AtomicOrdering::SeqCst);
        });

        scheduler.tick().await; // tick 1
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 0);

        scheduler.tick().await; // tick 2
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);

        scheduler.tick().await; // tick 3
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1); // no repeat
    }

    #[tokio::test]
    async fn test_repeating_task() {
        let scheduler = Scheduler::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        scheduler.schedule_repeating(1, 2, move || {
            counter_clone.fetch_add(1, AtomicOrdering::SeqCst);
        });

        scheduler.tick().await; // tick 1: fires
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);

        scheduler.tick().await; // tick 2: no fire
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);

        scheduler.tick().await; // tick 3: fires again
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let scheduler = Scheduler::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let handle = scheduler.schedule_delayed(1, move || {
            counter_clone.fetch_add(1, AtomicOrdering::SeqCst);
        });

        scheduler.cancel(&handle);
        scheduler.tick().await;
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 0);
    }
}
