/// This is a general example of how you would typically set up a trigger
/// simulation.
use anyhow::Result;
use uom::si::f64::{Frequency, Time};
use uom::si::{
    frequency::hertz,
    time::{nanosecond, second},
};

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
    // Then, these is your trigger configuration. You get these from the ODB
    let prompt_window = Time::new::<nanosecond>(64.0 * 8.0);
    let wait_gate = Time::new::<nanosecond>(128.0 * 8.0);
    let drift_veto = Time::new::<nanosecond>(300.0 * 16.0);
    let dead_time = Time::new::<nanosecond>(211864.0 * 16.0);
    // ===========================================

    Ok(())
}
