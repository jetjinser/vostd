# Mutex Symbolic Refinement FSM

## Purpose

This is the identity-aware refinement of the strict regular-language model in
`mutex-regular-language.md`. It is an oracle for classifying concrete traces,
not a model inferred from existing traces. It does not reproduce every
internal `WaitQueue` transition.

The model accepts legal prefixes (`L+`) and reports the first forbidden
transition for illegal prefixes (`L-`). Unlike the strict regular language,
this refinement retains Mutex, thread, and guard identities so it can detect
wrong-owner and wrong-guard release.

## Semantic alphabet

```text
create(m)
lock_call(m, t)
lock_acquire(m, t, g)
try_lock_call(m, t)
try_lock_success(m, t, g)
try_lock_fail(m, t)
guard_drop(m, t, g)
```

Events retain Mutex identity `m`, thread identity `t`, and guard identity `g`.
`lock_call` and `try_lock_call` are invocation events; successful acquisition
events are return/linearization events. A blocking `lock_call` need not be
immediately followed by `lock_acquire`.

`guard_drop` represents the explicit consuming method currently used by the
Verus implementation. It is not automatically generated from lexical scope
exit in this first model.

## States

```text
U        Unlocked
H(t, g)  Held by thread t through guard g
ERROR    First protocol violation has occurred
```

The control-state shape is finite. For a bounded set of identities, this is an
ordinary finite-state machine.

Initial state:

```text
U
```

Lifecycle note: the first `create(m)` (`uninitialized` → `unlocked`) and any
use before creation live in the four-state Verus model
(`ostd/specs/sync/mutex_protocol.rs`), which is authoritative. The table below
assumes post-create; every `create(m)` event here is therefore a duplicate.

## Legal transitions

| Current state | Event | Next state | Meaning |
| --- | --- | --- | --- |
| `U` | `lock_call(m,t)` | `U` | Begin blocking acquisition |
| `U` | `lock_acquire(m,t,g)` | `H(t,g)` | `lock()` returns a guard |
| `U` | `try_lock_call(m,t)` | `U` | Begin non-blocking attempt |
| `U` | `try_lock_success(m,t,g)` | `H(t,g)` | `try_lock()` returns `Some` |
| `H(t,g)` | `try_lock_call(m,t2)` | `H(t,g)` | Attempt while held |
| `H(t,g)` | `try_lock_fail(m,t2)` | `H(t,g)` | Failure has no state effect |
| `H(t,g)` | `guard_drop(m,t,g)` | `U` | Matching guard releases Mutex |

The table allows `try_lock_call` from both `U` and `H`; the result event
determines success or failure.

For traces containing only completed operations, the core language is:

```text
U       --lock_acquire(t,g)----> H(t,g)
U       --try_lock_success(t,g)-> H(t,g)
H(t,g)  --try_lock_fail(t2)-----> H(t,g)
H(t,g)  --guard_drop(t,g)-------> U
```

## Forbidden transitions

| Current state | Event | Violation |
| --- | --- | --- |
| `U` | `guard_drop(m,t,g)` | `unlock_without_lock` |
| `H(t,g)` | `guard_drop(m,t2,g)`, `t2 != t` | `wrong_owner_unlock` |
| `H(t,g)` | `guard_drop(m,t,g2)`, `g2 != g` | `wrong_guard_unlock` |
| `H(t,g)` | `lock_acquire(m,t2,g2)` | `acquire_while_held` |
| `H(t,g)` | `try_lock_success(m,t2,g2)` | `try_lock_succeeded_while_held` |
| any state | `create(m)` after creation | `duplicate_create` |

After entering `ERROR`, preserve the first violation and event index. Do not
classify later events because the trace is already outside the protocol.

## L+ examples

```text
create(m)
lock_call(m,t0)
lock_acquire(m,t0,g0)
guard_drop(m,t0,g0)
```

```text
create(m)
try_lock_call(m,t0)
try_lock_success(m,t0,g0)
try_lock_call(m,t1)
try_lock_fail(m,t1)
guard_drop(m,t0,g0)
```

```text
create(m)
lock_acquire(m,t0,g0)
guard_drop(m,t0,g0)
lock_acquire(m,t1,g1)
guard_drop(m,t1,g1)
```

## L- examples

### Unlock without a held guard

```text
create(m)
guard_drop(m,t0,g0)
```

The violation is `unlock_without_lock` at the second event.

### Double unlock

```text
create(m)
lock_acquire(m,t0,g0)
guard_drop(m,t0,g0)
guard_drop(m,t0,g0)
```

At the protocol-state level, the second release is
`unlock_without_lock`. A trace format that tracks consumed guard objects may
refine it to `use_after_guard_drop`.

### Wrong-owner unlock

```text
create(m)
lock_acquire(m,t0,g0)
guard_drop(m,t1,g0)
```

The violation is `wrong_owner_unlock`.

### Acquisition while held

```text
create(m)
lock_acquire(m,t0,g0)
lock_acquire(m,t1,g1)
```

The violation is `acquire_while_held`. A `lock_call` by `t1` while the Mutex
is held is not itself illegal because the real `lock()` API blocks.

## Modeling decisions

### Blocking

The FSM models the eventual result of `lock()`, not the complete sleep/wake
protocol. A blocked `lock_call` may remain pending while another thread
releases the Mutex.

### Fairness and scheduling

The model does not constrain which waiter acquires the Mutex after a release,
and does not model starvation or FIFO ordering.

### Ownership

Ownership is retained because the guard is `!Send` and the protocol should be
able to identify a wrong-owner release. If trace extraction cannot recover
thread identity, ownership checks may temporarily be marked `unknown`, but
should not be removed from the model.

### Data access

`Deref` and `DerefMut` are not separate lock transitions. They are operations
permitted while the corresponding guard is live. The first model does not
track the protected value.

### Internal state

The atomic `lock` bit and `PointsTo<T>` resource are implementation evidence
for `U` and `H(t,g)`. They are not separate events in this API-level language.

## Known limitations

1. The implementation uses explicit `guard.drop()` because ordinary Rust
   `Drop` implementations are commented out for Verus compatibility.
2. A completed `lock_acquire` is illegal when the lock is already held. A
   lower-level implementation model must represent failed attempts and
   waiting separately.
3. The relationship between `abstract_lock.rs` and this guard-based API needs
   differential testing, especially around unlock ownership and `locked`.
4. No top-level `kernel/` directory is present in the inspected checkout.

The implementation connection is present in `ostd/src/sync/mutex.rs`:

* `acquire_lock` checks the protocol transition for successful acquisition and
  failed `try_lock` at the atomic compare-exchange.
* `MutexGuard::drop` checks `held --guard_drop--> unlocked` using the guard's
  type invariant.

The lower-level `release_lock` atomic store is not separately related to the
old atomic value yet. Its current contract exposes the `PointsTo<T>` cell id,
but not enough information to prove that the atomic lock was previously
`true` from that permission alone. This remains a concrete next spec question,
not an assumption added to the proof.
