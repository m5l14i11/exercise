use std::sync::{Arc, Barrier};
use std::thread;

use admission::{AcquireError, ClusterConfig, ClusterPool, PoolConfig};

fn mock_pool(clusters: &[(&str, u64)]) -> Arc<ClusterPool> {
    ClusterPool::new(
        PoolConfig::default().group(
            "g",
            clusters
                .iter()
                .map(|(name, max)| ClusterConfig::new(*name, *max))
                .collect(),
        ),
    )
}

/// Run `n` threads that call try_acquire at the same instant.
fn race(pool: &Arc<ClusterPool>, n: usize) -> Vec<Result<admission::Permit, AcquireError>> {
    let barrier = Arc::new(Barrier::new(n));
    let handles: Vec<_> = (0..n)
        .map(|_| {
            let (pool, b) = (Arc::clone(pool), Arc::clone(&barrier));
            thread::spawn(move || {
                b.wait();
                pool.try_acquire("g")
            })
        })
        .collect();
    handles.into_iter().map(|h| h.join().unwrap()).collect()
}

#[test]
fn inc1() {
    for _ in 0..2000 {
        let pool = mock_pool(&[("trino-1", 1)]);
        let results = race(&pool, 8);

        let admitted = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(admitted, 1, "capacity 1 admitted {admitted}");
        assert_eq!(pool.running("g", "trino-1"), Some(1));

        drop(results);

        assert_eq!(pool.running("g", "trino-1"), Some(0));
        assert!(pool.try_acquire("g").is_ok());
    }
}

#[test]
fn inc2() {
    const N: u64 = 3;
    let pool = mock_pool(&[("trino-1", N)]);

    let _permits: Vec<_> = (0..N).map(|_| pool.try_acquire("g").unwrap()).collect();

    assert_eq!(pool.running("g", "trino-1"), Some(N));

    pool.set_max_running("g", "trino-1", 1);

    assert_eq!(pool.running("g", "trino-1"), Some(N));
    assert!(matches!(pool.try_acquire("g"), Err(AcquireError::Busy)));
}

#[test]
fn inc3() {
    const N: usize = 8;
    let names: Vec<String> = (0..N).map(|i| format!("c{i}")).collect();
    let spec: Vec<(&str, u64)> = names.iter().map(|n| (n.as_str(), 1)).collect();

    for round in 0..2000 {
        let pool = mock_pool(&spec);
        let results = race(&pool, N);

        let busy = results.iter().filter(|r| r.is_err()).count();
        assert_eq!(
            busy, 0,
            "INC-3 (round {round}): {N} free slots across the group, but {busy} callers got Busy"
        );
    }
}

#[test]
fn inc4() {
    const N: u64 = 1;
    let pool = mock_pool(&[("trino-1", N)]);

    let permit = pool.try_acquire("g").unwrap();
    assert_eq!(pool.running("g", "trino-1"), Some(N));

    drop(permit);

    assert_eq!(pool.running("g", "trino-1"), Some(0));

    let next = pool.try_acquire("g");
    assert!(next.is_ok(), "dropped permit should free the slot");
}
