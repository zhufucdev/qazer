use std::cmp::Ordering;
use std::time::Duration;
use tokio::time;

#[derive(Eq, PartialEq)]
struct WatchNode<ID: Eq> {
    id: ID,
    timer: Duration,
}

pub struct Watcher<ID: Eq> {
    pq: Vec<WatchNode<ID>>,
}

impl<ID: Eq> Watcher<ID> {
    pub fn new() -> Self {
        Self { pq: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            pq: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, id: ID, timer: Duration) {
        self._push(WatchNode { id, timer })
    }

    pub async fn next(&mut self) -> Option<ID> {
        let peek = self.pq.pop()?;
        time::sleep(peek.timer).await;
        for node in &mut self.pq {
            node.timer = if node.timer < peek.timer {
                Duration::default()
            } else {
                node.timer - peek.timer
            }
        }
        Some(peek.id)
    }

    fn _push(&mut self, node: WatchNode<ID>) {
        for idx in 0..self.pq.len() {
            if node < self.pq[idx] {
                self.pq.insert(idx, node);
                return;
            }
        }
        self.pq.push(node);
    }
}

impl<ID> FromIterator<(ID, Duration)> for Watcher<ID>
where
    ID: Eq,
    ID: Clone,
{
    fn from_iter<T: IntoIterator<Item = (ID, Duration)>>(iter: T) -> Self {
        let mut pq = Vec::from_iter(
            iter.into_iter()
                .map(|(id, dur)| WatchNode { id, timer: dur }),
        );
        pq.sort();
        Self { pq }
    }
}

impl<ID: Eq> PartialOrd<Self> for WatchNode<ID> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<ID: Eq> Ord for WatchNode<ID> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timer.cmp(&other.timer)
    }
}
