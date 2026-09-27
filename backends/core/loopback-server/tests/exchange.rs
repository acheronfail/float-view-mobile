use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use float_view_loopback_server::exchange::{Exchange, ExchangeError, OperationKind};
use float_view_loopback_server::limits::*;

#[test]
fn operation_admission_includes_drained_and_completed_work() {
    let exchange = Exchange::<usize, usize>::default();
    let generation = exchange.start().unwrap();
    let mut ids = Vec::new();
    for index in 0..MAX_OPERATIONS {
        ids.push(
            exchange
                .submit(
                    generation,
                    (index / MAX_CLIENT_OPERATIONS) as u64,
                    OperationKind::Read,
                    index,
                )
                .unwrap(),
        );
    }
    assert_eq!(
        exchange.submit(generation, 100, OperationKind::Read, 0),
        Err(ExchangeError::Full)
    );
    let batch = exchange.drain(generation, Duration::ZERO).unwrap();
    assert_eq!(batch.len(), MAX_HOST_BATCH);
    assert_eq!(
        batch.iter().map(|p| p.operation).collect::<Vec<_>>(),
        (0..MAX_HOST_BATCH).collect::<Vec<_>>()
    );
    assert_eq!(
        exchange.submit(generation, 100, OperationKind::Read, 0),
        Err(ExchangeError::Full)
    );
    exchange.complete(ids[0], 123).unwrap();
    assert_eq!(
        exchange.submit(generation, 100, OperationKind::Read, 0),
        Err(ExchangeError::Full)
    );
    assert_eq!(exchange.take_completion(ids[0]).unwrap(), Some(123));
    assert!(
        exchange
            .submit(generation, 100, OperationKind::Read, 0)
            .is_ok()
    );
}

#[test]
fn client_admission_is_independent_and_completion_is_terminal_once() {
    let exchange = Exchange::<(), u8>::default();
    let generation = exchange.start().unwrap();
    let id = exchange
        .submit(generation, 1, OperationKind::Read, ())
        .unwrap();
    assert_eq!(exchange.complete(id, 1), Err(ExchangeError::InvalidState));
    for _ in 1..MAX_CLIENT_OPERATIONS {
        exchange
            .submit(generation, 1, OperationKind::Read, ())
            .unwrap();
    }
    assert_eq!(
        exchange.submit(generation, 1, OperationKind::Read, ()),
        Err(ExchangeError::Full)
    );
    assert!(
        exchange
            .submit(generation, 2, OperationKind::Read, ())
            .is_ok()
    );
    exchange.drain(generation, Duration::ZERO).unwrap();
    assert_eq!(exchange.take_completion(id).unwrap(), None);
    exchange.complete(id, 42).unwrap();
    assert_eq!(exchange.complete(id, 43), Err(ExchangeError::InvalidState));
    assert_eq!(exchange.take_completion(id).unwrap(), Some(42));
    assert_eq!(exchange.take_completion(id), Err(ExchangeError::Stale));
    assert_eq!(exchange.complete(id, 44), Err(ExchangeError::Stale));
}

#[test]
fn client_loss_cancels_reads_but_retains_accepted_mutations_without_retry() {
    let exchange = Exchange::<&str, ()>::default();
    let generation = exchange.start().unwrap();
    let reading = exchange
        .submit(generation, 1, OperationKind::Read, "running read")
        .unwrap();
    exchange.drain(generation, Duration::ZERO).unwrap();
    let queued = exchange
        .submit(generation, 1, OperationKind::Read, "queued read")
        .unwrap();
    let mutation = exchange
        .submit(generation, 1, OperationKind::Mutation, "mutation")
        .unwrap();
    exchange.detach_client(generation, 1).unwrap();
    assert!(exchange.is_cancelled(reading));
    assert!(exchange.is_cancelled(queued));
    assert!(!exchange.is_cancelled(mutation));
    let batch = exchange.drain(generation, Duration::ZERO).unwrap();
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].id, mutation);
    assert_eq!(batch[0].operation, "mutation");
    exchange.complete(reading, ()).unwrap();
    exchange.complete(mutation, ()).unwrap();
    assert_eq!(
        exchange.take_completion(mutation),
        Err(ExchangeError::Stale)
    );
    assert!(
        exchange
            .drain(generation, Duration::ZERO)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn stale_generation_and_idempotent_destruction() {
    let exchange = Exchange::<(), ()>::default();
    let generation = exchange.start().unwrap();
    let old = exchange
        .submit(generation, 1, OperationKind::Mutation, ())
        .unwrap();
    exchange.drain(generation, Duration::ZERO).unwrap();
    exchange.stop().unwrap();
    exchange.stop().unwrap();
    assert!(exchange.is_cancelled(old));
    let next = exchange.start().unwrap();
    assert!(next > generation);
    let new = exchange.submit(next, 1, OperationKind::Read, ()).unwrap();
    assert_eq!(old.sequence, new.sequence);
    assert_eq!(exchange.complete(old, ()), Err(ExchangeError::Stale));
    assert!(!exchange.is_cancelled(new));
    exchange.destroy().unwrap();
    exchange.destroy().unwrap();
    assert_eq!(exchange.start(), Err(ExchangeError::Destroyed));
    assert_eq!(exchange.complete(new, ()), Err(ExchangeError::Destroyed));
}

#[test]
fn foreign_exchange_completion_cannot_alias_a_local_operation() {
    let left = Exchange::<(), ()>::default();
    let right = Exchange::<(), ()>::default();
    let left_generation = left.start().unwrap();
    let right_generation = right.start().unwrap();
    let foreign = left
        .submit(left_generation, 1, OperationKind::Read, ())
        .unwrap();
    let local = right
        .submit(right_generation, 1, OperationKind::Read, ())
        .unwrap();
    assert_eq!(foreign.sequence, local.sequence);
    right.drain(right_generation, Duration::ZERO).unwrap();
    assert_eq!(right.complete(foreign, ()), Err(ExchangeError::Stale));
    right.complete(local, ()).unwrap();
    assert_eq!(right.take_completion(local).unwrap(), Some(()));
}

#[test]
fn waiting_worker_releases_lock_for_submit_and_stop() {
    let exchange = Arc::new(Exchange::<(), ()>::default());
    let generation = exchange.start().unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let worker = {
        let exchange = exchange.clone();
        let barrier = barrier.clone();
        thread::spawn(move || {
            barrier.wait();
            loop {
                match exchange.drain(generation, MAX_HOST_WAIT) {
                    Ok(batch) => {
                        for operation in batch {
                            exchange.complete(operation.id, ()).unwrap();
                        }
                    }
                    Err(ExchangeError::Stopped) => break,
                    Err(error) => panic!("unexpected exchange failure: {error:?}"),
                }
            }
        })
    };
    barrier.wait();
    let id = exchange
        .submit(generation, 1, OperationKind::Read, ())
        .unwrap();
    let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
    while exchange.take_completion(id).unwrap().is_none() {
        assert!(
            Instant::now() < deadline,
            "worker retained a transport lock"
        );
        thread::yield_now();
    }
    let stopped = Instant::now();
    exchange.stop().unwrap();
    worker.join().unwrap();
    assert!(stopped.elapsed() < SHUTDOWN_TIMEOUT);
}

#[test]
fn wait_is_bounded_and_invalid_wait_is_rejected() {
    let exchange = Exchange::<(), ()>::default();
    let generation = exchange.start().unwrap();
    assert!(matches!(
        exchange.drain(generation, MAX_HOST_WAIT + Duration::from_millis(1)),
        Err(ExchangeError::InvalidWait)
    ));
    let start = Instant::now();
    assert!(
        exchange
            .drain(generation, MAX_HOST_WAIT)
            .unwrap()
            .is_empty()
    );
    assert!(start.elapsed() >= MAX_HOST_WAIT);
    assert!(start.elapsed() < SHUTDOWN_TIMEOUT);
}
