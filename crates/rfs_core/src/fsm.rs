//! Finite state machine

use std::fmt::Debug;

/// A type that implements this trait can undergo state machine transitions.
///
/// There are no outputs during state transitions, just changes in state.
/// This is sufficient for the purposes of this project.
pub trait TransitableState: Clone + Debug + Default {
    /// Events that can trigger a change in state.
    type Event;

    /// Process the input and modify the internal state, if applicable.
    fn ingest(&mut self, transition: Self::Event);
}

/// Generate the state transition logic.
///
/// This macro implements [TransitableState::ingest].
///
/// ```ignore
/// #[derive(Clone, Debug, Default)]
/// enum SimpleMachine {
///     #[default]
///     Off,
///     On,
///     Running,
/// }
/// enum SimpleMachineEvents {
///     PowerButtonPress,
///     Start,
///     Stop,
/// }
/// state_transitions! {
///     state: SimpleMachine;
///     event: SimpleMachineEvents;
///
///     Off + PowerButtonPress => On,
///     On + PowerButtonPress => Off,
///     On + Start => Running,
///     Running + Stop => On,
///     Running + PowerButtonPress => Off
/// }
/// ```
#[macro_export]
macro_rules! state_transitions {
    {
        type State = $st: ident;
        type Event = $ev: ident;

        $($st_variant: ident + $($ev_variant: ident)|+ => $new_st: ident;)*
    } => {

        impl TransitableState for $st {
            type Event = $ev;

            fn ingest(&mut self, transition: Self::Event) {

                *self = match (&self, transition) {

                    $(
                        (Self::$st_variant, $(Self::Event::$ev_variant)|+) => Self::$new_st,
                    )*

                    // all other cases
                    _ => self.clone(),
                };

            }
        }
    };
}

// below macro definition
pub(crate) use state_transitions;

#[cfg(test)]
mod macro_tests {

    use std::default;

    use super::*;

    #[derive(Clone, Debug, Default)]
    enum SimpleMachine {
        #[default]
        Off,
        On,
        Running,
    }

    enum SimpleMachineEvents {
        PowerButtonPress,

        Start,

        Stop,
    }

    state_transitions! {
        type State = SimpleMachine;
        type Event = SimpleMachineEvents;

        Off + PowerButtonPress => On;
        On + PowerButtonPress => Off;
        On + Start => Running;
        Running + Stop => On;
        Running + PowerButtonPress => Off;
    }

    #[derive(Clone, Debug, Default)]
    enum OtherMachine {
        #[default]
        This,
    }

    enum OtherMachineEvents {}

    // state machine with 1 state and no transitions
    state_transitions! {
        type State = OtherMachine;
        type Event = OtherMachineEvents;
    }

    #[test]
    fn test_state_transitions() {
        let mut machine = SimpleMachine::default();

        machine.ingest(SimpleMachineEvents::PowerButtonPress);
        assert!(matches!(machine, SimpleMachine::On));

        machine.ingest(SimpleMachineEvents::Start);
        assert!(matches!(machine, SimpleMachine::Running));

        machine.ingest(SimpleMachineEvents::Start);
        assert!(matches!(machine, SimpleMachine::Running));

        machine.ingest(SimpleMachineEvents::Stop);
        assert!(matches!(machine, SimpleMachine::On));

        machine.ingest(SimpleMachineEvents::Start);
        assert!(matches!(machine, SimpleMachine::Running));

        machine.ingest(SimpleMachineEvents::PowerButtonPress);
        assert!(matches!(machine, SimpleMachine::Off));
    }
}
