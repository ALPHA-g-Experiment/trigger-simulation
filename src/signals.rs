use uom::si::f64::Time;

#[derive(Clone, Copy, Debug)]
/// A [`Signal`] represents the possible signals that go into the MLU. These are
/// the output of passing digitized anode wire waveforms through a
/// discriminator.
///
/// The inner [`Time`] represents the time of arrival of the signal.
pub enum Signal {
    /// The first/actual signal from a cosmic event.
    Cosmic(Time),
    /// The first/actual from a pbar annihilation event.
    Pbar(Time),
    /// Secondary afterpulse after an event caused by ionization electrons
    /// originating from the drift region of the rTPC.
    Afterpulse(Time),
}

impl Signal {
    /// Returns the time of arrival of the signal.
    pub fn time(&self) -> Time {
        match self {
            Signal::Cosmic(t) => *t,
            Signal::Pbar(t) => *t,
            Signal::Afterpulse(t) => *t,
        }
    }
}
