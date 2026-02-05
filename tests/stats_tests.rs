//! Tests for stats recording with ID integrity under concurrency.

use std::path::PathBuf;
use std::sync::Arc;

use edgelord::core::db::run_migrations;
use edgelord::core::service::statistics::{RecordedOpportunity, StatsRecorder};
use rust_decimal_macros::dec;

/// Guard to clean up temporary database file after test
struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Test that record_opportunity returns correct IDs under concurrency.
///
/// This test verifies that when multiple opportunities are recorded concurrently,
/// each call returns the correct unique ID that matches the inserted row,
/// not a race condition where max(id) returns the wrong value.
#[test]
fn record_opportunity_returns_correct_id_under_concurrency() {
    // Set up temporary file-based database with WAL mode for better concurrency
    // Use a unique name to avoid conflicts between test runs
    let temp_file = std::env::temp_dir().join(format!("test_stats_{}.db", std::process::id()));
    let db_url = format!("sqlite://{}", temp_file.display());
    
    // Create pool with larger size to handle concurrent connections
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    let db_pool = Pool::builder()
        .max_size(15) // Larger pool for concurrent access
        .build(manager)
        .unwrap();
    
    // Enable WAL mode on initial connection (database-level setting)
    {
        use diesel::prelude::*;
        let mut conn = db_pool.get().unwrap();
        diesel::sql_query("PRAGMA journal_mode=WAL")
            .execute(&mut conn)
            .unwrap();
    }
    
    run_migrations(&db_pool).unwrap();
    let recorder = StatsRecorder::new(db_pool.clone());
    
    // Clean up temp file after test
    let _guard = TempFileGuard(temp_file);

    // Number of concurrent inserts
    const NUM_THREADS: usize = 10;
    let recorder = Arc::new(recorder);
    let mut handles = Vec::new();

    // Spawn concurrent threads that insert opportunities
    // Small random delay to avoid all threads hitting the DB at exactly the same time
    for i in 0..NUM_THREADS {
        let recorder_clone = recorder.clone();
        let handle = std::thread::spawn(move || {
            // Small delay to stagger inserts slightly
            std::thread::sleep(std::time::Duration::from_millis(i as u64 * 10));
            
            let event = RecordedOpportunity {
                strategy: format!("strategy-{}", i),
                market_ids: vec![format!("market-{}", i)],
                edge: dec!(0.05),
                expected_profit: dec!(1.0),
                executed: false,
                rejected_reason: None,
            };
            
            recorder_clone.record_opportunity(&event)
        });
        handles.push(handle);
    }

    // Collect all returned IDs
    let mut returned_ids: Vec<i32> = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .flatten()
        .collect();

    // Verify we got IDs for all inserts
    assert!(
        returned_ids.len() > 0,
        "At least some inserts should return IDs"
    );

    // Verify all returned IDs are unique
    returned_ids.sort();
    let unique_count = returned_ids.len();
    returned_ids.dedup();
    assert_eq!(
        returned_ids.len(),
        unique_count,
        "All returned IDs should be unique"
    );
    
    assert_eq!(
        returned_ids.len(),
        NUM_THREADS,
        "All inserts should return IDs (got {} unique IDs)",
        returned_ids.len()
    );

    // Verify IDs match what's actually in the database
    use diesel::prelude::*;
    use edgelord::core::db::schema::opportunities;
    let mut conn = db_pool.get().unwrap();
    let db_ids: Vec<i32> = opportunities::table
        .select(opportunities::id)
        .load::<Option<i32>>(&mut conn)
        .unwrap()
        .into_iter()
        .flatten()
        .collect();

    assert_eq!(db_ids.len(), NUM_THREADS, "Database should have all records");
    
    // Verify returned IDs match database IDs
    let mut db_ids_sorted = db_ids;
    db_ids_sorted.sort();
    assert_eq!(
        returned_ids, db_ids_sorted,
        "Returned IDs should match database IDs"
    );
}
