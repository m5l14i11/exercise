# Notes

## Part 1: Incidents

| Incident | Root cause | Reproducing test | Fix |
|----------|------------|------------------|-----|
| INC-1 | try_increment() uses a non-atomic load/check/store sequence. Concurrent callers can both observe available capacity and both succeed, losing an increment and violating the configured concurrency limit. | run concurrent acquisitions and assert exactly one succeeds while its permit remains alive.| Replace load/check/store with a compare-and-swap |
| INC-2 | available() calculates max_running - running using unsigned arithmetic. Lowering capacity below the current running count causes underflow. | Acquire 3 permits, lower the maximum to 1, and verify new acquisitions return Busy without panicking; existing running count remains 3. | Use max_running.saturating_sub(running) so available capacity never becomes negative or wraps around. |
| INC-3 | try_acquire() selects a cluster from a potentially stale availability snapshot and immediately returns Busy if try_increment() fails. Another caller may have claimed that cluster's slot while other clusters still have capacity. | Create 8 clusters with capacity 1 each; concurrently acquire 8 permits and assert all succeed and every cluster is used. Repeat across multiple rounds to stress contention. | Try remaining candidates when a reservation fails, rather than returning Busy immediately. Prefer clusters with more available capacity; no cursor is required. Return Busy only after all candidates have been tried without success. |
| INC-4 | Permit has no automatic cleanup. If a query exits without explicitly calling release(), its running-count reservation leaks, eventually exhausting capacity.| Acquire the only slot, drop the permit without calling release(), assert the running count returns to zero, and verify another acquisition succeeds. | Implement Drop for Permit to decrement the exact reserved ClusterState. Store an Arc<ClusterState> in the permit so configuration reloads cannot cause it to decrement a replacement cluster's counter. |

## Part 2: Performance

Machine (M1 Pro 16GB 8 Core (6 Performance and 2 Efficiency)):

| Bench | Before | After |
|-------|--------|-------|
| acquire_release/1  | 626.69 ns | |
| acquire_release/4  | 5.4169 µs | |
| acquire_release/16 | 28.749 µs | |

Changes, and why each one helps (or didn't):

-

Selection policy, and why:

## Part 3: Waiting for a slot

## Part 4: Global admission across 6 pods

## Bonus: INC-5 and INC-6

## Before shipping, I would still want

-
