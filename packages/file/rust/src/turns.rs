//! The queue of the requests of one session in one process (SF-3.2, SF-3.8).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use binding_core::Cancel;
use polyspec_session_store_core::Error;

/// The tickets of one identifier in the order of their requests; the first holds the turn.
#[derive(Debug, Default)]
struct Queue {
    next: u64,
    order: VecDeque<u64>,
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
    ticket: u64,
}

fn poisoned<T>(_error: PoisonError<T>) -> Error {
    Error::Internal { what: "the queue of the sessions was poisoned by a panic" }
}

impl Turns {
    /// The queues; a poisoned lock still gives them, because no code panics while it changes them.
    fn queues(&self) -> MutexGuard<'_, HashMap<String, Queue>> {
        match self.queues.lock() {
            Ok(queues) => queues,
            Err(error) => error.into_inner(),
        }
    }

    /// Takes a ticket out of the queue of an identifier and wakes the other waiters.
    fn leave(&self, id: &str, ticket: u64) {
        let mut queues = self.queues();
        if let Some(queue) = queues.get_mut(id) {
            queue.order.retain(|held| *held != ticket);
            if queue.order.is_empty() {
                queues.remove(id);
            }
        }
        drop(queues);
        self.changed.notify_all();
    }

    /// Waits, first come first served, for the turn of an identifier (SF-3.2). A cancellation wakes the wait at once
    /// and takes the request out of the queue with the kind `cancelled` (SF-3.8).
    pub(crate) fn enter(self: &Arc<Self>, id: &str, cancel: Option<&Cancel>) -> Result<Turn, Error> {
        let ticket = {
            let mut queues = self.queues.lock().map_err(poisoned)?;
            let queue = queues.entry(id.to_owned()).or_default();
            let ticket = queue.next;
            queue.next =
                queue.next.checked_add(1).ok_or(Error::Internal { what: "the tickets of a session pass u64" })?;
            queue.order.push_back(ticket);
            ticket
        };
        // The turn exists from here, so a failure below gives the ticket back when the turn drops.
        let turn = Turn { turns: Arc::clone(self), id: id.to_owned(), ticket };
        // The wake takes the lock of the queues before it notifies, so it cannot fall between the check of the
        // cancellation and the wait, which hold that lock.
        let _guard = cancel.map(|cancel| {
            let turns = Arc::clone(self);
            cancel.on_cancel(move || {
                drop(turns.queues());
                turns.changed.notify_all();
            })
        });
        let mut queues = self.queues.lock().map_err(poisoned)?;
        loop {
            if cancel.is_some_and(Cancel::is_cancelled) {
                drop(queues);
                return Err(Error::Cancelled);
            }
            match queues.get(id).and_then(|queue| queue.order.front()) {
                Some(front) if *front == ticket => return Ok(turn),
                Some(_) => {}
                None => return Err(Error::Internal { what: "the queue of a waiting session is gone" }),
            }
            queues = self.changed.wait(queues).map_err(poisoned)?;
        }
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        self.turns.leave(&self.id, self.ticket);
    }
}
