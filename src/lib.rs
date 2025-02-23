use crate::gen::{EventGenerator, WireEvent};
use std::iter::Peekable;

/// Utilities to generate input data for the trigger system.
pub mod gen;
/// Memory Lookup Unit.
pub mod mlu;

type InnerGen<T> = Box<dyn EventGenerator<Time = T, Item = WireEvent<T>>>;

struct Generator<T> {
    inner: Vec<Peekable<InnerGen<T>>>,
}

// Deriving `Default` would only work for `T: Default`.
impl<T> Default for Generator<T> {
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}

impl<T: PartialOrd> Iterator for Generator<T> {
    type Item = WireEvent<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let (index, _) = self
            .inner
            .iter_mut()
            // Safe to unwrap because we only keep useful generators.
            .map(|g| g.peek().unwrap())
            .enumerate()
            .min_by(|(_, a), (_, b)| a.time.partial_cmp(&b.time).unwrap())?;

        let next_event = self.inner[index].next();
        if self.inner[index].peek().is_none() {
            let _ = self.inner.swap_remove(index);
        }

        next_event
    }
}

#[derive(bon::Builder)]
pub struct World<T> {
    #[builder(field)]
    gen: Generator<T>,
}

impl<T, S: world_builder::State> WorldBuilder<T, S> {
    /// Add an event generator to the [`World`].
    pub fn add_generator<G>(mut self, gen: G) -> Self
    where
        G: EventGenerator<Time = T> + 'static,
    {
        let mut peekable = (Box::new(gen) as InnerGen<T>).peekable();
        // Only keep around useful generators.
        if peekable.peek().is_some() {
            self.gen.inner.push(peekable);
        }
        self
    }
}
