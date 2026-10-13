#!/usr/bin/env python3
"""On-disk SQLite query-plan audit of production SQL (no external dependencies)."""
import argparse
import json
import re
import sqlite3
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def measure(db, sql, args):
    plan = [row[3] for row in db.execute('EXPLAIN QUERY PLAN ' + sql, args)]
    start = time.perf_counter()
    rows = db.execute(sql, args).fetchall()
    return dict(ms=round((time.perf_counter()-start)*1000, 3), rows=len(rows), plan=plan)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=int, default=200000)
    parser.add_argument('--locations', type=int, default=2000)
    args = parser.parse_args()
    assert 0 < args.samples <= 2000000 and 0 < args.locations <= 20000
    with tempfile.TemporaryDirectory(prefix='float-view-sql-scale-') as root:
        db = sqlite3.connect(Path(root)/'session.sqlite')
        db.executescript((ROOT/'backends/sql/session.sql').read_text())
        db.executemany("INSERT INTO sample(sequence, observed_at_ms, monotonic_elapsed_ms, ride_state, speed_kph) VALUES (?, ?, ?, 'riding', ?)", ((n,n*100,n*100,n%50) for n in range(1,args.samples+1)))
        db.executemany('INSERT INTO location(location_sequence, observed_at_ms, monotonic_elapsed_ms, latitude, longitude) VALUES (?, ?, ?, -34.9, 138.6)', ((n,n*10000,n*10000) for n in range(1,args.locations+1)))
        db.commit()
        source = (ROOT/'backends/core/session-sqlite/src/pruner.rs').read_text()
        sql = re.search(r'"(DELETE FROM location WHERE NOT EXISTS[^"\n]+)"', source)[1]
        # Measure matching without mutating, then the production delete in a rolled-back transaction.
        select = sql.replace('DELETE FROM location', 'SELECT location_sequence FROM location', 1)
        bindings = {'1': 1000} if '?1' in sql else [1000]
        result = measure(db, select, bindings)
        assert result['rows'] == max(0, args.locations - (args.samples*100+1000)//10000)
        print(json.dumps(dict(operation='prune_location_neighborhood', samples=args.samples, locations=args.locations, **result)))
        db.execute('BEGIN')
        start = time.perf_counter()
        db.execute(sql, bindings)
        print(json.dumps(dict(operation='prune_location_delete', ms=round((time.perf_counter()-start)*1000,3))))
        db.rollback()
        for name, sql, params in [
            ('sample_tail', 'SELECT sequence FROM sample ORDER BY monotonic_elapsed_ms DESC,sequence DESC LIMIT 1', []),
            ('location_dedup', 'SELECT EXISTS(SELECT 1 FROM location WHERE monotonic_elapsed_ms=? AND latitude=? AND longitude=?)', [10000,-34.9,138.6]),
        ]:
            print(json.dumps(dict(operation=name, **measure(db,sql,params))))

if __name__ == '__main__':
    main()
