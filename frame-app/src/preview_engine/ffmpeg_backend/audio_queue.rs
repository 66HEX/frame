use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AudioQueuePoll {
    Admitted(usize),
    Pending(Option<Instant>),
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AudioQueueOutcome {
    Admitted(usize),
    Cancelled,
    Failed,
}
