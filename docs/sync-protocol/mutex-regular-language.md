# Mutex Regular Language

## Purpose

This document gives the strict finite-alphabet version of the Mutex protocol.
It is the core language that should be presented as the regular-language
model. Object, thread, and guard identities are intentionally projected away.

Identity-sensitive checks are described separately as a symbolic refinement in
`mutex-fsm.md`.

## Finite alphabet

```text
Σ_mutex = {
    create,
    lock_call,
    lock_acquire,
    try_lock_call,
    try_lock_success,
    try_lock_fail,
    guard_drop
}
```

The alphabet is finite. An implementation trace is projected onto this
alphabet by removing arguments:

```text
lock_acquire(m, t, g)  ↦ lock_acquire
guard_drop(m, t, g)    ↦ guard_drop
try_lock_fail(m, t)    ↦ try_lock_fail
```

## Automaton

States:

```text
Q = { U, H, ERROR }
```

Initial state:

```text
q0 = U
```

Lifecycle note: the first `create` (`uninitialized` → `unlocked`) and any
use before creation live in the four-state Verus model
(`ostd/specs/sync/mutex_protocol.rs`), which is authoritative. The three-state
tables below assume post-create; every `create` event here is therefore a
duplicate.

The legal transition relation is:

| State | Event | Next state |
| --- | --- | --- |
| `U` | `lock_call` | `U` |
| `U` | `lock_acquire` | `H` |
| `U` | `try_lock_call` | `U` |
| `U` | `try_lock_success` | `H` |
| `U` | `try_lock_fail` | `U` |
| `H` | `lock_call` | `H` |
| `H` | `try_lock_call` | `H` |
| `H` | `try_lock_fail` | `H` |
| `H` | `guard_drop` | `U` |

The following events enter `ERROR`:

| State | Event | Violation |
| --- | --- | --- |
| `U` | `guard_drop` | `unlock_without_lock` |
| `H` | `lock_acquire` | `acquire_while_held` |
| `H` | `try_lock_success` | `try_lock_succeeded_while_held` |
| `U` or `H` | `create` | `duplicate_create` |

The error state is absorbing for checking purposes. The checker reports the
first event that enters it.

## Regular expression for the completed-operation core

If invocation events are omitted and `A` abbreviates either successful
acquisition event:

```text
A = lock_acquire | try_lock_success
F = try_lock_fail
D = guard_drop
```

then the sequential completed-operation language is:

```text
L+_core = create ( A F* D )*
```

Examples in the language:

```text
create
create lock_acquire guard_drop
create try_lock_success try_lock_fail try_lock_fail guard_drop
create lock_acquire guard_drop try_lock_success guard_drop
create lock_acquire guard_drop lock_acquire guard_drop
```

This expression describes the single-object sequential abstraction. The FSM
above is the authoritative definition when invocation events and pending
blocking operations are retained.

## Positive and negative languages

`L+_mutex` is the set of finite event sequences whose run never enters
`ERROR`. It is regular because the run is recognized by the finite automaton
above.

For testing, define `L-_mutex` as the set of illegal prefixes whose final
event is the first event entering `ERROR`. This is also regular: each error
kind is represented by a finite transition to a labeled error state.

This definition is intentionally a safety-language definition. It does not
claim that every sequence outside `L+_mutex` is a useful negative test, nor
does it model fairness or liveness.

## Projection and refinement

The strict regular language is obtained from a concrete trace by projection:

```text
π : concrete_event → Σ_mutex
```

The relationship between the two models is:

```text
concrete trace
      │
      ├── π: erase m/t/g ──> regular FSM ──> L+ or L-
      │
      └── symbolic checker ─> ownership / guard-identity findings
```

For example, both of these project to the same regular trace:

```text
lock_acquire(m,t0,g0) guard_drop(m,t0,g0)
lock_acquire(m,t0,g0) guard_drop(m,t1,g0)
```

The regular language accepts both because both look like:

```text
lock_acquire guard_drop
```

The symbolic refinement accepts only the first and reports
`wrong_owner_unlock` for the second. This is intentional: ownership is not a
regular-language property after identities are erased.

## Scope boundary

Included:

```text
exclusive acquisition
failed try-lock as a state-preserving event
release after acquisition
repeated sequential use
first safety violation
```

Excluded:

```text
fairness
starvation
FIFO ordering
memory ordering
exact WaitQueue behavior
thread/guard identity
```
