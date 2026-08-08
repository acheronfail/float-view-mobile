# Bundled SQLite identity

`infra-sqlite-storage` locks `rusqlite` 0.37.0 and `libsqlite3-sys` 0.35.0 with
the `bundled` feature. It compiles the SQLite amalgamation from the Cargo
source; no Apple SQLite module, Android `SQLiteDatabase`, `pkg-config`, or
system library fallback is enabled.

The macOS and physical arm64 Android smoke executables both reported SQLite
3.50.2 with source ID:

```text
2025-06-28 14:00:48 2af157d77fb1304a74176eaee7fbc7c7e932d946bf25325e9c26c91db19e3079
```

The common compile options were:

```text
ATOMIC_INTRINSICS=1
DEFAULT_AUTOVACUUM
DEFAULT_CACHE_SIZE=-2000
DEFAULT_FILE_FORMAT=4
DEFAULT_FOREIGN_KEYS
DEFAULT_JOURNAL_SIZE_LIMIT=-1
DEFAULT_MMAP_SIZE=0
DEFAULT_PAGE_SIZE=4096
DEFAULT_PCACHE_INITSZ=20
DEFAULT_RECURSIVE_TRIGGERS
DEFAULT_SECTOR_SIZE=4096
DEFAULT_SYNCHRONOUS=2
DEFAULT_WAL_AUTOCHECKPOINT=1000
DEFAULT_WAL_SYNCHRONOUS=2
DEFAULT_WORKER_THREADS=0
DIRECT_OVERFLOW_READ
ENABLE_API_ARMOR
ENABLE_COLUMN_METADATA
ENABLE_DBSTAT_VTAB
ENABLE_FTS3
ENABLE_FTS3_PARENTHESIS
ENABLE_FTS5
ENABLE_LOAD_EXTENSION
ENABLE_MEMORY_MANAGEMENT
ENABLE_RTREE
ENABLE_STAT4
HAVE_ISNAN
MALLOC_SOFT_LIMIT=1024
MAX_ATTACHED=10
MAX_COLUMN=2000
MAX_COMPOUND_SELECT=500
MAX_DEFAULT_PAGE_SIZE=8192
MAX_EXPR_DEPTH=1000
MAX_FUNCTION_ARG=1000
MAX_LENGTH=1000000000
MAX_LIKE_PATTERN_LENGTH=50000
MAX_MMAP_SIZE=0x7fff0000
MAX_PAGE_COUNT=0xfffffffe
MAX_PAGE_SIZE=65536
MAX_SQL_LENGTH=1000000000
MAX_TRIGGER_DEPTH=1000
MAX_VARIABLE_NUMBER=32766
MAX_VDBE_OP=250000000
MUTEX_PTHREADS
SOUNDEX
SYSTEM_MALLOC
THREADSAFE=1
USE_URI
```

The target compiler is recorded by SQLite (`clang-17.0.0` on macOS and
`clang-21.0.0` in NDK 28.2). The bundled configuration also selects
`TEMP_STORE=1`, `MAX_WORKER_THREADS=8` on macOS and `TEMP_STORE=3`,
`MAX_WORKER_THREADS=0` on Android. These target-appropriate differences do not
alter the required WAL, foreign-key, interruption, or file-format semantics.
The smoke executable prints the complete sorted option set so embedded iOS and
future Android matrix jobs compare runtime identity rather than relying on this
record alone.

The canonical catalog lifecycle, WAL write/read, checkpoint, close/reopen, and
`sqlite3_interrupt` path pass on macOS. The same lifecycle executable passes on
physical Android; its source builds for all four Android ABIs and iOS device
and simulator architectures.
