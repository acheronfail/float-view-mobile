use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use float_view_storage::{engine_identity, open_catalog};

fn main() -> Result<(), Box<dyn Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!(
        "float-view-storage-smoke-{}-{nonce}.sqlite",
        std::process::id()
    ));
    let connection = open_catalog(&path)?;
    let identity = engine_identity(&connection)?;
    connection.execute(
        "INSERT INTO saved_device(saved_device_id, device_history_id, created_at_ms) VALUES ('saved', 'history', 1)",
        [],
    )?;
    connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    connection.close().map_err(|(_, error)| error)?;

    let reopened = open_catalog(&path)?;
    let rows: i64 =
        reopened.query_row("SELECT count(*) FROM saved_device", [], |row| row.get(0))?;
    reopened.close().map_err(|(_, error)| error)?;
    std::fs::remove_file(&path)?;
    let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
    let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));

    println!(
        "sqlite={} source={} options={} rows={rows}",
        identity.version,
        identity.source_id,
        identity.compile_options.join(",")
    );
    Ok(())
}
