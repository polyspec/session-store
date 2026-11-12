//! The queue of the requests of one session in one process (SR-3.2): a fair tokio mutex per identifier.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::OwnedMutexGuard;

use polyspec_sessions_core::Error as CoreError;

/// The queues of every identifier of one store.
#[derive(Debug, Default)]
pub(crate) struct Turns {
    queues: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

/// The turn of one request; dropping it lets the next request of the identifier go on.
#[derive(Debug)]
pub(crate) struct Turn {
    _guard: OwnedMutexGuard<()>,
}

impl Turns {
    /// Waits, first come first served, for the turn of an identifier. tokio's mutex queues its waiters in order.
    pub(crate) async fn enter(&self, id: &str) -> Result<Turn, CoreError> {
        let queue = {
            let mut queues = self
                .queues
                .lock()
                .map_err(|_error| CoreError::Internal { what: "the queue of the sessions was poisoned by a panic" })?;
            // A queue that no request holds or waits for is removed, so the map holds only identifiers in use.
            queues.retain(|_, queue| Arc::strong_count(queue) > 1);
            Arc::clone(queues.entry(id.to_owned()).or_default())
        };
        Ok(Turn { _guard: queue.lock_owned().await })
    }
}
