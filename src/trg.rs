use uom::si::f64::Time;

/// The source of a [`WireEvent`].
#[derive(Clone, Copy, Debug)]
pub enum Source {
    /// First avalanche identifying a cosmic event.
    Cosmic,
    /// First avalanche identifying an anti-proton event.
    Pbar,
    /// Secondary avalanches e.g. originally from the drift region.
    Afterpulse,
}

/// A [`WireEvent`] represents an input signal to the trigger system.
///
/// The digitized anode wire waveforms go into digital discriminators. This
/// discriminator outputs ([`WireEvent`]s) are then sent to the trigger system.
#[derive(Clone, Copy, Debug)]
pub struct WireEvent {
    pub source: Source,
    pub time: Time,
}
