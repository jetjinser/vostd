# Mutex protocol model

This directory contains the first complete synchronization-protocol result.

```text
mutex-inventory.md          actual API and spec investigation
mutex-regular-language.md   strict finite-alphabet regular language
mutex-fsm.md                identity-aware symbolic refinement
```

The Verus-side model is:

```text
ostd/specs/sync/mutex_protocol.rs
```

It defines the finite event alphabet, protocol states, the pure `step`
function, trace execution, and small positive/negative proof witnesses.

Run its targeted verification with:

```text
VERUS_Z3_PATH=$(command -v z3) VERUS_USE_RUSTUP=0 \
  cargo dv focus --targets ostd -- \
  --verify-only-module specs::sync::mutex_protocol
```

The regular model answers:

> Is this sequence a legal Mutex protocol sequence?

The symbolic refinement additionally answers:

> Does this guard belong to this Mutex and its current owner?

The second question is deliberately kept separate because it requires data
identities that are erased by the finite regular-language projection.
