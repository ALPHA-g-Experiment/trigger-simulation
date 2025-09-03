use crate::gen::{EventGenerator, Generator, Positive, WireEvent};
use crate::mlu::{LookupTable, Mlu, TrgSignal};
use bon::bon;
use std::ops::Add;

/// Utilities to generate input data for the trigger system.
pub mod gen;
/// Memory Lookup Unit.
pub mod mlu;

/// A trait that defines the interface for an observer of the trigger system.
///
/// The default implementation of all methods is a no-op. Users are expected to
/// override the methods they are interested in.
#[allow(unused_variables)]
pub trait Observer {
    type Time;

    /// Called when a new [`WireEvent`] is generated.
    fn on_wire_event(&mut self, event: &WireEvent<Self::Time>) {}
    /// Called when a signal goes into the TRG box (i.e. output of the MLU).
    fn on_trg_in(&mut self, signal: &TrgSignal<Self::Time>) {}
    /// Called when a TRG signal is suppressed by the drift veto.
    fn on_trg_drift_veto(&mut self, signal: &TrgSignal<Self::Time>) {}
    /// Called when a TRG signal is suppressed by the scaledown.
    fn on_trg_scaledown(&mut self, signal: &TrgSignal<Self::Time>) {}
    /// Called when a TRG signal is suppressed by the dead time.
    fn on_trg_dead_time(&mut self, signal: &TrgSignal<Self::Time>) {}
    /// Called when a trigger signal is sent to the DAQ.
    fn on_trg_out(&mut self, signal: &TrgSignal<Self::Time>) {}
}

pub struct World<T, O> {
    generator: Generator<T>,
    mlu: Mlu<T>,
    drift_veto: Positive<T>,
    scaledown: u32,
    dead_time: Positive<T>,
    observer: O,
    // Inner state of the TRG box
    veto_until: Option<T>,
    busy_until: Option<T>,
    counter: u32,
    // Each wire event "flushes" the TRG box. Meaning that the "current" event
    // is ahead of the "current" TRG signal.
    // This allows us to keep the observer "time-aware" i.e. it can assume that
    // everything happens in the correct order.
    prev_event: Option<WireEvent<T>>,
}

#[bon]
impl<T, O> World<T, O> {
    #[builder]
    pub fn new(
        #[builder(field)] generator: Generator<T>,
        prompt_window: Positive<T>,
        wait_gate: Positive<T>,
        lookup_table: LookupTable,
        drift_veto: Positive<T>,
        scaledown: u32,
        dead_time: Positive<T>,
        observer: O,
    ) -> Self {
        let mlu = Mlu::new(prompt_window, wait_gate, lookup_table);

        Self {
            generator,
            mlu,
            drift_veto,
            scaledown,
            dead_time,
            observer,
            veto_until: None,
            busy_until: None,
            counter: 0,
            prev_event: None,
        }
    }
}

impl<T, O, S: world_builder::State> WorldBuilder<T, O, S> {
    /// Add an event generator to the [`World`].
    pub fn add_generator<G>(mut self, gen: G) -> Self
    where
        G: EventGenerator<Time = T> + 'static,
    {
        self.generator.add_generator(gen);
        self
    }
}

impl<T, O> World<T, O>
where
    T: Add<Output = T> + PartialOrd + Clone,
    O: Observer<Time = T>,
{
    /// Run a simulation of the trigger system until all generators are
    /// exhausted. Note that if any of the provided generators are infinite,
    /// this method will run forever.
    pub fn run(mut self) -> O {
        for event in self.generator {
            // Needed for time-aware observers
            if let Some(e) = self.prev_event {
                self.observer.on_wire_event(&e);
            }
            self.prev_event = Some(event.clone());

            let Some(trg_signal) = self.mlu.process(&event) else {
                continue;
            };
            self.observer.on_trg_in(&trg_signal);

            if let Some(veto_until) = &self.veto_until {
                if trg_signal.time <= *veto_until {
                    self.observer.on_trg_drift_veto(&trg_signal);
                    continue;
                }
            }
            self.veto_until = Some(trg_signal.time.clone() + self.drift_veto.inner().clone());

            if self.counter != self.scaledown {
                self.observer.on_trg_scaledown(&trg_signal);
                self.counter += 1;
                continue;
            }
            self.counter = 0;

            if let Some(busy_until) = &self.busy_until {
                if trg_signal.time <= *busy_until {
                    self.observer.on_trg_dead_time(&trg_signal);
                    continue;
                }
            }
            self.observer.on_trg_out(&trg_signal);
            self.busy_until = Some(trg_signal.time + self.dead_time.inner().clone());
        }
        // Needed for time-aware observers
        if let Some(e) = self.prev_event {
            self.observer.on_wire_event(&e);
        }

        self.observer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gen::*;
    use std::iter::repeat;

    #[derive(Default)]
    struct TestObserver {
        events: Vec<WireEvent<i32>>,
        trg_in: Vec<TrgSignal<i32>>,
        drift_veto: Vec<TrgSignal<i32>>,
        scaledown: Vec<TrgSignal<i32>>,
        dead_time: Vec<TrgSignal<i32>>,
        trg_out: Vec<TrgSignal<i32>>,
    }

    impl Observer for TestObserver {
        type Time = i32;

        fn on_wire_event(&mut self, event: &WireEvent<Self::Time>) {
            self.events.push(*event);
        }

        fn on_trg_in(&mut self, signal: &TrgSignal<Self::Time>) {
            self.trg_in.push(*signal);
        }

        fn on_trg_drift_veto(&mut self, signal: &TrgSignal<Self::Time>) {
            self.drift_veto.push(*signal);
        }

        fn on_trg_scaledown(&mut self, signal: &TrgSignal<Self::Time>) {
            self.scaledown.push(*signal);
        }

        fn on_trg_dead_time(&mut self, signal: &TrgSignal<Self::Time>) {
            self.dead_time.push(*signal);
        }

        fn on_trg_out(&mut self, signal: &TrgSignal<Self::Time>) {
            self.trg_out.push(*signal);
        }
    }

    #[test]
    fn world_generator() {
        let noise1 = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(0)
            .duration(Positive::new(3).unwrap())
            .inter_arrival_time(repeat(Positive::new(2).unwrap()))
            .wire_pattern(repeat(WirePattern::from_bits(1)))
            .build();
        let noise2 = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(1)
            .duration(Positive::new(3).unwrap())
            .inter_arrival_time(repeat(Positive::new(2).unwrap()))
            .wire_pattern(repeat(WirePattern::from_bits(1)))
            .build();

        let observer = World::builder()
            .add_generator(noise1)
            .add_generator(noise2)
            .prompt_window(Positive::new(100).unwrap())
            .wait_gate(Positive::new(100).unwrap())
            .lookup_table(LookupTable::default())
            .drift_veto(Positive::new(100).unwrap())
            .scaledown(0)
            .dead_time(Positive::new(100).unwrap())
            .observer(TestObserver::default())
            .build()
            .run();

        assert_eq!(
            observer
                .events
                .into_iter()
                .map(|e| e.time)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn world_prompt_window() {
        let noise = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(-100)
            .duration(Positive::new(10).unwrap())
            .inter_arrival_time(repeat(Positive::new(2).unwrap()))
            .wire_pattern(repeat(WirePattern::from_bits(1)))
            .build();
        let observer = World::builder()
            .add_generator(noise)
            .prompt_window(Positive::new(3).unwrap())
            .wait_gate(Positive::new(100).unwrap())
            .lookup_table(LookupTable::from([WirePattern::from_bits(1)]))
            .drift_veto(Positive::new(100).unwrap())
            .scaledown(0)
            .dead_time(Positive::new(100).unwrap())
            .observer(TestObserver::default())
            .build()
            .run();
        assert_eq!(
            observer
                .trg_in
                .into_iter()
                .map(|s| s.time)
                .collect::<Vec<_>>(),
            vec![-95]
        );
    }

    #[test]
    fn world_wait_gate() {
        let noise = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(0)
            .duration(Positive::new(45).unwrap())
            .inter_arrival_time(repeat(Positive::new(10).unwrap()))
            .wire_pattern(repeat(WirePattern::from_bits(1)))
            .build();
        let observer = World::builder()
            .add_generator(noise)
            .prompt_window(Positive::new(9).unwrap())
            .wait_gate(Positive::new(2).unwrap())
            .lookup_table(LookupTable::from([WirePattern::from_bits(1)]))
            .drift_veto(Positive::new(100).unwrap())
            .scaledown(0)
            .dead_time(Positive::new(100).unwrap())
            .observer(TestObserver::default())
            .build()
            .run();
        assert_eq!(
            observer
                .trg_in
                .into_iter()
                .map(|s| s.time)
                .collect::<Vec<_>>(),
            vec![19, 39]
        );
    }

    #[test]
    fn world_lookup_table() {
        let noise = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(0)
            .inter_arrival_time(repeat(Positive::new(3).unwrap()))
            .wire_pattern(vec![
                WirePattern::from_bits(1),
                WirePattern::from_bits(2),
                WirePattern::from_bits(1),
                WirePattern::from_bits(2),
            ])
            .build();
        let observer = World::builder()
            .add_generator(noise)
            .prompt_window(Positive::new(1).unwrap())
            .wait_gate(Positive::new(1).unwrap())
            .lookup_table(LookupTable::from([WirePattern::from_bits(1)]))
            .drift_veto(Positive::new(100).unwrap())
            .scaledown(0)
            .dead_time(Positive::new(100).unwrap())
            .observer(TestObserver::default())
            .build()
            .run();
        assert_eq!(
            observer
                .trg_in
                .into_iter()
                .map(|s| s.time)
                .collect::<Vec<_>>(),
            vec![4, 10]
        );
    }

    #[test]
    fn world_drift_veto() {
        let noise = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(0)
            .duration(Positive::new(21).unwrap())
            .inter_arrival_time(repeat(Positive::new(4).unwrap()))
            .wire_pattern(repeat(WirePattern::from_bits(1)))
            .build();
        let observer = World::builder()
            .add_generator(noise)
            .prompt_window(Positive::new(1).unwrap())
            .wait_gate(Positive::new(1).unwrap())
            .lookup_table(LookupTable::from([WirePattern::from_bits(1)]))
            .drift_veto(Positive::new(5).unwrap())
            .scaledown(0)
            .dead_time(Positive::new(7).unwrap())
            .observer(TestObserver::default())
            .build()
            .run();
        assert_eq!(
            observer
                .drift_veto
                .into_iter()
                .map(|s| s.time)
                .collect::<Vec<_>>(),
            vec![9, 17]
        );
    }

    #[test]
    fn world_scaledown() {
        let noise = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(0)
            .duration(Positive::new(25).unwrap())
            .inter_arrival_time(repeat(Positive::new(4).unwrap()))
            .wire_pattern(repeat(WirePattern::from_bits(1)))
            .build();
        let observer = World::builder()
            .add_generator(noise)
            .prompt_window(Positive::new(1).unwrap())
            .wait_gate(Positive::new(1).unwrap())
            .lookup_table(LookupTable::from([WirePattern::from_bits(1)]))
            .drift_veto(Positive::new(5).unwrap())
            .scaledown(1)
            .dead_time(Positive::new(7).unwrap())
            .observer(TestObserver::default())
            .build()
            .run();
        assert_eq!(
            observer
                .scaledown
                .into_iter()
                .map(|s| s.time)
                .collect::<Vec<_>>(),
            vec![5, 21]
        );
    }

    #[test]
    fn world_dead_time() {
        let noise = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(0)
            .duration(Positive::new(21).unwrap())
            .inter_arrival_time(repeat(Positive::new(4).unwrap()))
            .wire_pattern(repeat(WirePattern::from_bits(1)))
            .build();
        let observer = World::builder()
            .add_generator(noise)
            .prompt_window(Positive::new(1).unwrap())
            .wait_gate(Positive::new(1).unwrap())
            .lookup_table(LookupTable::from([WirePattern::from_bits(1)]))
            .drift_veto(Positive::new(1).unwrap())
            .scaledown(0)
            .dead_time(Positive::new(7).unwrap())
            .observer(TestObserver::default())
            .build()
            .run();
        assert_eq!(
            observer
                .dead_time
                .into_iter()
                .map(|s| s.time)
                .collect::<Vec<_>>(),
            vec![9, 17]
        );
    }

    #[test]
    fn world_trg_out() {
        let noise = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(0)
            .duration(Positive::new(21).unwrap())
            .inter_arrival_time(repeat(Positive::new(4).unwrap()))
            .wire_pattern(repeat(WirePattern::from_bits(1)))
            .build();
        let observer = World::builder()
            .add_generator(noise)
            .prompt_window(Positive::new(1).unwrap())
            .wait_gate(Positive::new(1).unwrap())
            .lookup_table(LookupTable::from([WirePattern::from_bits(1)]))
            .drift_veto(Positive::new(1).unwrap())
            .scaledown(0)
            .dead_time(Positive::new(7).unwrap())
            .observer(TestObserver::default())
            .build()
            .run();
        assert_eq!(
            observer
                .trg_out
                .into_iter()
                .map(|s| s.time)
                .collect::<Vec<_>>(),
            vec![5, 13]
        );
    }

    #[derive(Default)]
    struct TimeAwareObserver {
        last_event: Option<WireEvent<i32>>,
    }

    impl Observer for TimeAwareObserver {
        type Time = i32;

        fn on_wire_event(&mut self, event: &WireEvent<Self::Time>) {
            self.last_event = Some(*event);
        }

        fn on_trg_in(&mut self, signal: &TrgSignal<Self::Time>) {
            let last_event_time = self.last_event.unwrap().time;
            assert!(signal.time > last_event_time);
        }
    }

    #[test]
    fn world_time_correctness() {
        let noise = SecondaryGenerator::builder()
            .source(Source::Noise)
            .origin(0)
            .duration(Positive::new(7).unwrap())
            .inter_arrival_time(repeat(Positive::new(3).unwrap()))
            .wire_pattern(repeat(WirePattern::from_bits(1)))
            .build();
        let _ = World::builder()
            .add_generator(noise)
            .prompt_window(Positive::new(1).unwrap())
            .wait_gate(Positive::new(1).unwrap())
            .lookup_table(LookupTable::from([WirePattern::from_bits(1)]))
            .drift_veto(Positive::new(1).unwrap())
            .scaledown(0)
            .dead_time(Positive::new(2).unwrap())
            .observer(TimeAwareObserver::default())
            .build()
            .run();
    }
}
