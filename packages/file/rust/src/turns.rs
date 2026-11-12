//! The queue of the requests of one session in one process (SF-3.2, SR-3.2).

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use polyspec_sessions_core::Error as CoreError;

/// The tickets of one identifier: the next ticket to give and the ticket whose turn it is.
#[derive(Debug, Default)]
struct Queue {
    next: u64,
    serving: u64,
}

/// The turns of every identifier of one store.
#[derive(Debug, Default)]
pub(crate) struct Turns {
    queues: Mutex<HashMap<String, Queue>>,
    changed: Condvar,
}

/// The turn of one request; dropping it lets the next request of the identifier go on.
#[derive(Debug)]
pub(crate) struct Turn {
    turns: Arc<Turns>,
    id: String,
}

fn poisoned<T>(_error: std::sync::PoisonError<T>) -> CoreError {
    CoreError::Internal { what: "the queue of the sessions was poisoned by a panic" }
}

impl Turns {
    /// Waits, first come first served, for the turn of an identifier (SF-3.2).
    pub(crate) fn enter(self: &Arc<Self>, id: &str) -> Result<Turn, CoreError> {
        let mut queues: MutexGuard<'_, HashMap<String, Queue>> = self.queues.lock().map_err(poisoned)?;
        let queue = queues.entry(id.to_owned()).or_default();
        let ticket = queue.next;
        queue.next =
            queue.next.checked_add(1).ok_or(CoreError::Internal { what: "the tickets of a session pass u64" })?;
        loop {
            match queues.get(id) {
                Some(queue) if queue.serving == ticket => break,
                Some(_) => {}
                None => return Err(CoreError::Internal { what: "the queue of a waiting session is gone" }),
            }
            queues = self.changed.wait(queues).map_err(poisoned)?;
        }
        Ok(Turn { turns: Arc::clone(self), id: id.to_owned() })
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        // A poisoned queue still ends the turn: the counters stay valid, since no code panics while it changes them.
        let mut queues = match self.turns.queues.lock() {
            Ok(queues) => queues,
            Err(error) => error.into_inner(),
        };
        let done = match queues.get_mut(&self.id) {
            Some(queue) => {
                // serving < next while a turn is held, and next is at most u64::MAX.
                queue.serving += 1;
                queue.serving == queue.next
            }
            None => false,
        };
        if done {
            queues.remove(&self.id);
        }
        self.turns.changed.notify_all();
    }
}
