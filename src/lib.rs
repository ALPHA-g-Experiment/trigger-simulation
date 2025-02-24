use crate::gen::{EventGenerator, Generator, Positive};
use crate::mlu::{LookupTable, Mlu};
use bon::bon;

/// Utilities to generate input data for the trigger system.
pub mod gen;
/// Memory Lookup Unit.
pub mod mlu;

pub struct World<T> {
    generator: Generator<T>,
    mlu: Mlu<T>,
    drift_veto: Positive<T>,
    scaledown: u32,
    dead_time: Positive<T>,
}

#[bon]
impl<T> World<T> {
    #[builder]
    pub fn new(
        #[builder(field)] generator: Generator<T>,
        prompt_window: Positive<T>,
        wait_gate: Positive<T>,
        lookup_table: LookupTable,
        drift_veto: Positive<T>,
        scaledown: u32,
        dead_time: Positive<T>,
    ) -> Self {
        let mlu = Mlu::new(prompt_window, wait_gate, lookup_table);

        Self {
            generator,
            mlu,
            drift_veto,
            scaledown,
            dead_time,
        }
    }
}

impl<T, S: world_builder::State> WorldBuilder<T, S> {
    /// Add an event generator to the [`World`].
    pub fn add_generator<G>(mut self, gen: G) -> Self
    where
        G: EventGenerator<Time = T> + 'static,
    {
        self.generator.add_generator(gen);
        self
    }
}
