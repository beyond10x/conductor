//! The read conductord's readiness asks its store (`story:daemon-probes-and-metrics`).

use super::Store;

impl Store {
    /// Whether the database answers one read now: the first record of this handle's tenant's
    /// feed, whatever it holds. A read that fails is not kept among the handle's failures
    /// ([`Store::check`]): the daemon asks every tick, on a handle that writes nothing.
    #[must_use]
    pub fn answers(&self) -> bool {
        self.runtime
            .block_on(self.log.read_feed(&self.tenant, 0, 1))
            .is_ok()
    }
}
