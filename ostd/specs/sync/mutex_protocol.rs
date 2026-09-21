use vstd::prelude::*;

verus! {

/// The finite alphabet of the API-level Mutex protocol.
#[derive(PartialEq, Eq, Clone, Copy)]
pub ghost enum Event {
    Create,
    LockCall,
    LockAcquire,
    TryLockCall,
    TryLockSuccess,
    TryLockFail,
    GuardDrop,
}

/// States of the finite regular-language model.
#[derive(PartialEq, Eq, Clone, Copy)]
pub ghost enum State {
    Uninitialized,
    Unlocked,
    Held,
    Error,
}

/// Abstraction of the concrete atomic lock bit.
pub open spec fn state_of_lock_bit(locked: bool) -> State {
    if locked {
        State::Held
    } else {
        State::Unlocked
    }
}

/// One transition of the API-level Mutex automaton.
pub open spec fn step(state: State, event: Event) -> State {
    match state {
        State::Uninitialized => {
            match event {
                Event::Create => State::Unlocked,
                _ => State::Error,
            }
        },
        State::Unlocked => {
            match event {
                Event::LockCall | Event::TryLockCall | Event::TryLockFail => {
                    State::Unlocked
                },
                Event::LockAcquire | Event::TryLockSuccess => State::Held,
                Event::Create | Event::GuardDrop => State::Error,
            }
        },
        State::Held => {
            match event {
                Event::LockCall | Event::TryLockCall | Event::TryLockFail => State::Held,
                Event::GuardDrop => State::Unlocked,
                Event::Create | Event::LockAcquire | Event::TryLockSuccess => State::Error,
            }
        },
        State::Error => State::Error,
    }
}

/// Run a finite trace from the initial state.
pub open spec fn run(trace: Seq<Event>) -> State
{
    trace.fold_left_alt(State::Uninitialized, |state: State, event: Event|
        step(state, event))
}

/// The regular-language acceptance predicate.
pub open spec fn accepts(trace: Seq<Event>) -> bool {
    run(trace) != State::Error
}

/// A basic positive-language witness.
pub proof fn lemma_basic_lplus()
    ensures
        run(seq![Event::Create, Event::LockAcquire, Event::GuardDrop])
            == State::Unlocked,
{
    assert(run(seq![]) == State::Uninitialized) by {
        reveal(run);
    }
    assert(run(seq![Event::Create]) == State::Unlocked) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(run(seq![Event::Create, Event::LockAcquire]) == State::Held) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(run(seq![Event::Create, Event::LockAcquire, Event::GuardDrop])
        == State::Unlocked) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
}

/// A failed try-lock preserves the held state.
pub proof fn lemma_try_lock_failure_lplus()
    ensures
        run(
            seq![
                Event::Create,
                Event::LockAcquire,
                Event::TryLockFail,
                Event::GuardDrop,
            ],
        ) == State::Unlocked,
{
    assert(run(seq![]) == State::Uninitialized) by {
        reveal(run);
    }
    assert(run(seq![Event::Create]) == State::Unlocked) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(run(seq![Event::Create, Event::LockAcquire]) == State::Held) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(
        run(seq![Event::Create, Event::LockAcquire, Event::TryLockFail])
            == State::Held
    ) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(
        run(
            seq![
                Event::Create,
                Event::LockAcquire,
                Event::TryLockFail,
                Event::GuardDrop,
            ],
        ) == State::Unlocked
    ) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 5);
        reveal(step);
    }
}

/// A release without a prior successful acquisition is rejected.
pub proof fn lemma_unlock_without_lock_lminus()
    ensures
        run(seq![Event::Create, Event::GuardDrop]) == State::Error,
{
    assert(run(seq![]) == State::Uninitialized) by {
        reveal(run);
    }
    assert(run(seq![Event::Create]) == State::Unlocked) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(run(seq![Event::Create, Event::GuardDrop]) == State::Error) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
}

/// A second completed acquisition is rejected while the Mutex is held.
pub proof fn lemma_acquire_while_held_lminus()
    ensures
        run(seq![Event::Create, Event::LockAcquire, Event::LockAcquire])
            == State::Error,
{
    assert(run(seq![]) == State::Uninitialized) by {
        reveal(run);
    }
    assert(run(seq![Event::Create]) == State::Unlocked) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(run(seq![Event::Create, Event::LockAcquire]) == State::Held) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(
        run(seq![Event::Create, Event::LockAcquire, Event::LockAcquire])
            == State::Error
    ) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
}

/// A successful try-lock behaves like a lock acquisition.
pub proof fn lemma_try_lock_success_lplus()
    ensures
        run(seq![Event::Create, Event::TryLockSuccess, Event::GuardDrop])
            == State::Unlocked,
{
    assert(run(seq![]) == State::Uninitialized) by {
        reveal(run);
    }
    assert(run(seq![Event::Create]) == State::Unlocked) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(run(seq![Event::Create, Event::TryLockSuccess]) == State::Held) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(
        run(seq![Event::Create, Event::TryLockSuccess, Event::GuardDrop])
            == State::Unlocked
    ) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
}

/// Two consecutive lock/unlock cycles stay within the protocol.
pub proof fn lemma_double_cycle_lplus()
    ensures
        run(
            seq![
                Event::Create,
                Event::LockAcquire,
                Event::GuardDrop,
                Event::LockAcquire,
                Event::GuardDrop,
            ],
        ) == State::Unlocked,
{
    assert(run(seq![]) == State::Uninitialized) by {
        reveal(run);
    }
    assert(run(seq![Event::Create]) == State::Unlocked) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(run(seq![Event::Create, Event::LockAcquire]) == State::Held) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(
        run(seq![Event::Create, Event::LockAcquire, Event::GuardDrop])
            == State::Unlocked
    ) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(
        run(
            seq![
                Event::Create,
                Event::LockAcquire,
                Event::GuardDrop,
                Event::LockAcquire,
            ],
        ) == State::Held
    ) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 5);
        reveal(step);
    }
    assert(
        run(
            seq![
                Event::Create,
                Event::LockAcquire,
                Event::GuardDrop,
                Event::LockAcquire,
                Event::GuardDrop,
            ],
        ) == State::Unlocked
    ) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 6);
        reveal(step);
    }
}

/// Any acquisition before creation is rejected.
pub proof fn lemma_use_before_create_lminus()
    ensures
        run(seq![Event::LockAcquire]) == State::Error,
{
    assert(run(seq![]) == State::Uninitialized) by {
        reveal(run);
    }
    assert(run(seq![Event::LockAcquire]) == State::Error) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
}

/// A second creation is rejected while the Mutex exists.
pub proof fn lemma_duplicate_create_lminus()
    ensures
        run(seq![Event::Create, Event::Create]) == State::Error,
{
    assert(run(seq![]) == State::Uninitialized) by {
        reveal(run);
    }
    assert(run(seq![Event::Create]) == State::Unlocked) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    assert(run(seq![Event::Create, Event::Create]) == State::Error) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
}

/// L+ collection: every positive witness is accepted.
pub proof fn lemma_lplus_accepted_all()
    ensures
        accepts(seq![Event::Create])
            && accepts(
                seq![Event::Create, Event::LockAcquire, Event::GuardDrop],
            )
            && accepts(
                seq![
                    Event::Create,
                    Event::LockAcquire,
                    Event::TryLockFail,
                    Event::GuardDrop,
                ],
            )
            && accepts(
                seq![
                    Event::Create,
                    Event::TryLockSuccess,
                    Event::GuardDrop,
                ],
            )
            && accepts(
                seq![
                    Event::Create,
                    Event::LockAcquire,
                    Event::GuardDrop,
                    Event::LockAcquire,
                    Event::GuardDrop,
                ],
            ),
{
    lemma_basic_lplus();
    lemma_try_lock_failure_lplus();
    lemma_try_lock_success_lplus();
    lemma_double_cycle_lplus();
    assert(run(seq![Event::Create]) == State::Unlocked) by {
        reveal(run);
        reveal_with_fuel(Seq::fold_left_alt, 4);
        reveal(step);
    }
    reveal(accepts);
}

/// L- collection: every negative witness is rejected.
pub proof fn lemma_lminus_rejected_all()
    ensures
        !accepts(seq![Event::Create, Event::GuardDrop])
            && !accepts(
                seq![
                    Event::Create,
                    Event::LockAcquire,
                    Event::LockAcquire,
                ],
            )
            && !accepts(seq![Event::LockAcquire])
            && !accepts(seq![Event::Create, Event::Create]),
{
    lemma_unlock_without_lock_lminus();
    lemma_acquire_while_held_lminus();
    lemma_use_before_create_lminus();
    lemma_duplicate_create_lminus();
    reveal(accepts);
}

} // verus!
