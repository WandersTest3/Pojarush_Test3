pub struct Scheduler<T> {
    now: u64,
    tasks: Vec<(u64, u64, T)>,
}

impl<T: Clone> Default for Scheduler<T> {
    fn default() -> Self {
        Self { now: 0, tasks: Vec::new() }
    }
}

impl<T: Clone> Scheduler<T> {
    pub fn after(&mut self, ticks: u64, task: T) {
        self.tasks.push((self.now + ticks.max(1), 0, task));
    }

    pub fn every(&mut self, ticks: u64, task: T) {
        let every = ticks.max(1);
        self.tasks.push((self.now + every, every, task));
    }

    pub fn clear(&mut self) {
        self.tasks.clear();
    }

    pub fn tick(&mut self, tick: u64) -> Vec<T> {
        self.now = tick;
        let mut due = Vec::new();
        self.tasks.retain_mut(|(at, every, task)| {
            if *at > tick {
                return true;
            }
            due.push(task.clone());
            if *every > 0 {
                *at = tick + *every;
                true
            } else {
                false
            }
        });
        due
    }
}
