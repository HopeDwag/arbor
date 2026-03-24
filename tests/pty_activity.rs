use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[test]
fn test_atomic_timestamp_works() {
    let ts = Arc::new(AtomicU64::new(0));
    assert_eq!(ts.load(Ordering::Relaxed), 0);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    ts.store(now, Ordering::Relaxed);
    assert!(ts.load(Ordering::Relaxed) > 0);
}
