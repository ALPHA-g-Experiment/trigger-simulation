use anyhow::Result;
use rand::seq::IndexedRandom;
use rand_distr::{Bernoulli, Beta, Distribution, Exp, Gamma};
use std::{iter::repeat, sync::LazyLock};
use trg::gen::{Positive, PrimaryGenerator, SecondaryGenerator, Source, WireEvent, WirePattern};
use trg::mlu::{LookupTable, TrgSignal};
use trg::{Observer, World};
use uom::si::f64::{Frequency, Time};
use uom::si::{
    frequency::hertz,
    time::{nanosecond, second},
};

const MLU_INTERESTING: u16 = 2;
const MLU_NOT_INTERESTING: u16 = 1;

static COSMIC_AFTERPULSES: LazyLock<Vec<Vec<f64>>> = LazyLock::new(|| {
    let contents = std::fs::read_to_string("../data/cosmic_afterpulses.json").unwrap();
    serde_json::from_str(&contents).unwrap()
});

#[derive(Default)]
struct MyObserver {
    events: u32,
    trg_in: u32,
    drift_veto: u32,
    trg_out: u32,
}

impl Observer for MyObserver {
    type Time = Time;

    fn on_wire_event(&mut self, event: &WireEvent<Self::Time>) {
        if matches!(event.source, Source::PrimaryPbar) {
            self.events += 1;
        }
    }

    fn on_trg_in(&mut self, _signal: &TrgSignal<Self::Time>) {
        self.trg_in += 1;
        self.drift_veto += 1;
    }

    fn on_trg_drift_veto(&mut self, _signal: &TrgSignal<Self::Time>) {
        self.drift_veto -= 1;
    }

    fn on_trg_out(&mut self, _signal: &TrgSignal<Self::Time>) {
        self.trg_out += 1;
    }
}

/// This is a general example of how you would typically set up a trigger
/// simulation.
fn main() -> Result<()> {
    // ===========================================
    // These are most likely your free parameters:
    let duration = Time::new::<second>(1000.0);
    let signal_rate = Frequency::new::<hertz>(100.0);
    // ===========================================

    // ===========================================
    // Then, these are parameters you estimated either
    // experimentally/theoretically/simulated:
    let bkg_observed = 434120.0;
    let bkg_time_window = 1385.0;
    let bkg_passed_mlu = 21949.0;
    let bkg_total_mlu = 103763.0;

    let signal_passed_mlu = 55993.0;
    let signal_total_mlu = 75520.0;
    // ===========================================

    // ===========================================
    // Then, this is your trigger configuration. You get these from the ODB
    let prompt_window = Time::new::<nanosecond>(64.0 * 8.0);
    let wait_gate = Time::new::<nanosecond>(128.0 * 8.0);
    let drift_veto = Time::new::<nanosecond>(300.0 * 16.0);
    let dead_time = Time::new::<nanosecond>(211864.0 * 16.0);
    // ===========================================

    // ===========================================
    // Actual simulation code:
    let pass_mlu = Beta::new(bkg_passed_mlu + 1.0, bkg_total_mlu - bkg_passed_mlu + 1.0)?
        .sample(&mut rand::rng());
    let bkg_rate = Frequency::new::<hertz>(
        Gamma::new(bkg_observed + 1.0, 1.0 / (bkg_time_window + 0.0))?.sample(&mut rand::rng()),
    );

    let bkg_gen = PrimaryGenerator::builder()
        .source(Source::PrimaryCosmic)
        .origin(Time::new::<second>(0.0))
        .duration(Positive::new(duration).unwrap())
        .inter_arrival_time(
            Exp::new(bkg_rate.get::<hertz>())?
                .map(|delta| Positive::new(Time::new::<second>(delta)).unwrap())
                .sample_iter(rand::rng()),
        )
        .wire_pattern(
            Bernoulli::new(pass_mlu)
                .unwrap()
                .map(|i| {
                    if i {
                        WirePattern::from_bits(MLU_INTERESTING)
                    } else {
                        WirePattern::from_bits(MLU_NOT_INTERESTING)
                    }
                })
                .sample_iter(rand::rng()),
        )
        .afterpulse(|event: &WireEvent<_>| {
            SecondaryGenerator::builder()
                .source(Source::SecondaryCosmic)
                .wire_pattern(repeat(event.wire_pattern))
                .inter_arrival_time(
                    COSMIC_AFTERPULSES
                        .choose(&mut rand::rng())
                        .unwrap()
                        .into_iter()
                        .map(|n| {
                            Positive::new(Time::new::<nanosecond>((*n as f64) * 16.0)).unwrap()
                        }),
                )
        })
        .build();

    let observer = World::builder()
        .add_generator(bkg_gen)
        .prompt_window(Positive::new(prompt_window).unwrap())
        .wait_gate(Positive::new(wait_gate).unwrap())
        .lookup_table(LookupTable::from([WirePattern::from_bits(MLU_INTERESTING)]))
        .drift_veto(Positive::new(drift_veto).unwrap())
        .scaledown(0)
        .dead_time(Positive::new(dead_time).unwrap())
        .observer(MyObserver::default())
        .build()
        .run();

    println!("Event counter: {}", observer.events);
    println!("Input counter: {}", observer.trg_in);
    println!("Drift veto counter: {}", observer.drift_veto);
    println!("Output counter: {}", observer.trg_out);

    Ok(())
}
