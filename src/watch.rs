use std::cmp::Ordering;
use std::collections::LinkedList;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;
use tokio::{select, time};

#[derive(Eq, PartialEq, Copy, Clone)]
struct WatchNode<ID>
where
    ID: Eq + Copy,
{
    id: ID,
    timer: Duration,
}

pub struct Watcher<ID>
where
    ID: Eq + Copy,
{
    pq: LinkedList<WatchNode<ID>>,
    change_chan: Sender<()>,
    pending: usize,
}

impl<ID> Watcher<ID>
where
    ID: Eq + Copy,
{
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1);
        Self {
            pq: LinkedList::new(),
            change_chan: tx,
            pending: 0,
        }
    }

    pub fn push(&mut self, id: ID, timer: Duration) {
        self._push(WatchNode { id, timer })
    }

    pub fn peek(&self) -> Option<ID> {
        self.pq.front().map(|e| e.id)
    }

    pub fn pop(&mut self) -> Option<ID> {
        if let Some(peek) = self.pq.pop_front() {
            let _ = self.change_chan.send(());
            Some(peek.id)
        } else {
            None
        }
    }

    pub async fn next(&mut self) -> Option<ID> {
        self.pending += 1;
        let mut rx = self.change_chan.subscribe();

        loop {
            let peek = self.pq.front()?.clone();
            select! {
                _ = time::sleep(peek.timer) => {
                    break
                },
                _ = rx.recv() => {}
            }
        }

        let peek = self.pq.pop_front()?.clone();
        for node in &mut self.pq {
            node.timer = if node.timer < peek.timer {
                Duration::default()
            } else {
                node.timer - peek.timer
            }
        }
        self.pending -= 1;
        Some(peek.id)
    }

    fn _push(&mut self, node: WatchNode<ID>) {
        for (idx, &curr) in self.pq.iter().enumerate() {
            if node < curr {
                let mut rear = self.pq.split_off(idx);
                rear.push_front(node);
                self.pq.append(&mut rear);
                if idx == 0 && self.pending > 0 {
                    let _ = self.change_chan.send(());
                }
                return;
            }
        }
        self.pq.push_front(node);
        let _ = self.change_chan.send(());
    }
}

impl<ID> FromIterator<(ID, Duration)> for Watcher<ID>
where
    ID: Eq + Copy
{
    fn from_iter<T: IntoIterator<Item = (ID, Duration)>>(iter: T) -> Self {
        let mut pq = Vec::from_iter(
            iter.into_iter()
                .map(|(id, dur)| WatchNode { id, timer: dur }),
        );
        pq.sort();
        
        let (tx, _) = broadcast::channel(1);
        Self {
            pq: LinkedList::from_iter(pq),
            change_chan: tx,
            pending: 0
        }
    }
}

impl<ID> PartialOrd<Self> for WatchNode<ID>
where
    ID: Eq + Copy,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<ID> Ord for WatchNode<ID>
where
    ID: Eq + Copy,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.timer.cmp(&other.timer)
    }
}
