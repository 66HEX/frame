//! Exercise the GPUI scheduler used on Linux and Windows after RNG upgrades.
#![cfg(any(target_os = "linux", target_os = "windows"))]

use gpui::{Priority, queue::PriorityQueueReceiver};

fn drain_all_priorities(spin: bool) {
    let (sender, mut receiver) = PriorityQueueReceiver::new();
    assert_eq!(receiver.try_pop().unwrap(), None);

    for sequence in 0..100 {
        for (lane, priority) in [Priority::High, Priority::Medium, Priority::Low]
            .into_iter()
            .enumerate()
        {
            sender.send(priority, (lane, sequence)).unwrap();
        }
    }

    // Each priority must preserve FIFO ordering and deliver every queued task once.
    let mut next_sequence = [0; 3];
    for _ in 0..300 {
        let (lane, sequence) = if spin {
            receiver.spin_try_pop().unwrap().unwrap()
        } else {
            receiver.pop().unwrap()
        };
        assert_eq!(sequence, next_sequence[lane]);
        next_sequence[lane] += 1;
    }
    assert_eq!(next_sequence, [100; 3]);
    assert_eq!(receiver.try_pop().unwrap(), None);

    drop(sender);
    assert!(receiver.try_pop().is_err());
}

#[test]
fn blocking_queue_preserves_tasks_and_priority_order() {
    drain_all_priorities(false);
}

#[test]
fn spinning_queue_preserves_tasks_and_priority_order() {
    drain_all_priorities(true);
}
