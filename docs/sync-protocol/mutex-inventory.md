# Mutex Protocol Inventory

## Scope

This document records the behavior visible at the `Mutex` API boundary. It
separates that behavior from the internal `WaitQueue` steps and the atomic
ghost proof. The first model covers one `Mutex<T>` instance and does not model
fairness, scheduling, or memory-ordering details.

## Implementation locations

| Concern | Location |
| --- | --- |
| `Mutex<T>` and `MutexGuard` | `ostd/src/sync/mutex.rs` |
| Blocking wait queue | `ostd/src/sync/wait.rs` |
| Detailed Mutex state machine | `ostd/specs/sync/mutex.rs` |
| Abstract lock specification | `ostd/specs/sync/abstract_lock.rs` |
| Failed `try_lock` regression test | `ostd/src/sync/mutex.rs` |

## Public API

### Construction

```text
Mutex::new(value) -> Mutex
```

Construction creates an unlocked mutex containing the protected value. The
protocol event is `create(m)`.

### Blocking acquisition

```text
Mutex::lock(&self) -> MutexGuard
```

The implementation calls `self.queue.wait_until(|| self.try_lock())`. At the
API level this is one operation that eventually returns a guard. It may
internally perform failed attempts, enqueue a waiter, sleep, wake, and retry.
The first model hides those implementation steps:

```text
lock_call(m, t) ... lock_acquire(m, t, g)
```

### Non-blocking acquisition

```text
Mutex::try_lock(&self) -> Option<MutexGuard>
```

The semantic outcomes are:

```text
try_lock_call(m, t)
try_lock_success(m, t, g)
```

or:

```text
try_lock_call(m, t)
try_lock_fail(m, t)
```

On failure, the mutex state does not change. In particular, a failed
`try_lock` must not create a temporary guard whose destruction unlocks the
mutex. The regression test in `mutex.rs` checks this behavior.

### Release

`Mutex::unlock` is private. The API-visible release operation is:

```text
MutexGuard::drop(self)
```

The guard consumes itself, returns its tracked `PointsTo<T>` permission to the
mutex, releases the atomic lock, and wakes one waiter.

The ordinary Rust `Drop` implementation is currently commented out because of
a Verus limitation. Therefore the first protocol trace must record an
explicit `guard.drop()` as the release event. A plain lexical end of scope or
`core::mem::drop(guard)` must not automatically be treated as release unless
the execution layer being traced has an equivalent drop hook.

Semantic event:

```text
guard_drop(m, t, g)
```

where `g` identifies the guard returned by the matching acquisition.

## Internal implementation events

These events exist in the implementation or detailed specification, but are
not part of the first API-level language:

```text
acquire_lock
release_lock
pre_check_lock
wait_until
enqueue_waker
enqueue_waker_inc_waker
check_lock
check_has_woken
wake_one
wake_one_loop
wake_up
```

They may be retained in a lower-level implementation trace and then
normalized into the API-level events above.

## Ownership and identity

The guard stores a reference to its associated Mutex and carries the tracked
permission for the protected value. `MutexGuard` is explicitly `!Send`.
Consequently, the first model should retain:

```text
mutex identity: m
thread identity: t
guard identity: g
```

The initial checker may use abstract owner names such as `t0` and `t1`, but it
should not erase ownership completely because that would hide wrong-owner
release errors.

## Existing client inventory

The current checkout contains Mutex uses in, among others:

```text
ostd/src/console.rs
ostd/src/smp.rs
ostd/src/mm/dma/
ostd/src/mm/frame/
ostd/src/mm/page_table/
ostd/src/arch/
```

The inspected checkout does not contain a top-level `kernel/` directory.
Kernel-side traces therefore need to be added later from the corresponding
source tree or execution environment.

## Existing specifications

The detailed Mutex specification models the implementation-level wait queue,
including the locked bit, waker sequence, per-thread program counters, and
stack frames. The abstract lock specification models a simpler protocol with
`start`, `lock`, `cs`, and `unlock` labels.

The abstract specification's `unlock` action currently requires the caller to
be at the `unlock` program point, but does not state `locked == true` or an
explicit owner condition in its local precondition. This is a candidate for
differential checking, not yet a confirmed specification bug: a higher-level
invariant may imply the missing condition.

## Normalization examples

### Successful acquisition

Source-level shape:

```text
let guard = m.lock();
use(&guard);
guard.drop();
```

Normalized trace:

```text
create(m)
lock_call(m, t0)
lock_acquire(m, t0, g0)
guard_drop(m, t0, g0)
```

### Failed try-lock while held

```text
let g0 = m.lock();
let result = m.try_lock();
g0.drop();
```

Normalized trace:

```text
lock_acquire(m, t0, g0)
try_lock_call(m, t1)
try_lock_fail(m, t1)
guard_drop(m, t0, g0)
```

The failed attempt leaves the state `Held(t0,g0)` unchanged.

### Invalid repeated release

```text
g0.drop();
g0.drop();
```

The second operation is not a valid Rust use of a consumed value, but is a
useful constructed protocol negative if the trace generator can represent
invalid client actions. It can later be refined as either
`double_unlock` or `use_after_guard_drop`.
