use anyhow::{Context, Result};
use rand::Rng;
use rand_distr::{Distribution, Exp};
use uom::si::f64::{Frequency, Time};
use uom::si::frequency::hertz;
use uom::si::time::second;

#[derive(Debug)]
/// An iterator over a Poisson process.
///
/// The iterator yields [`Time`]s at which events occur in a Poisson process.
/// Times are guaranteed to be in increasing order and their inter-arrival time
/// follows an exponential distribution with the specified rate parameter.
pub struct PoissonProcess<R> {
    time: Time,
    // [`Time`] doesn't implement the `num_traits::Float` trait, so we can't
    // directly sample [`Times`] from the exponential distribution. Instead,
    // we'll sample `f64`s in seconds and convert them before returning to the
    // user.
    exp: Exp<f64>,
    rng: R,
}

impl<R> PoissonProcess<R>
where
    R: Rng,
{
    /// Create a new Poisson process with the specified rate parameter.
    pub fn new(rate: Frequency, rng: R) -> Result<Self> {
        let lambda = rate.get::<hertz>();

        Ok(Self {
            time: Time::new::<second>(0.0),
            exp: Exp::new(lambda).context("failed to create exponential distribution")?,
            rng,
        })
    }
}

impl<R> Iterator for PoissonProcess<R>
where
    R: Rng,
{
    type Item = Time;

    fn next(&mut self) -> Option<Self::Item> {
        let delta_t = self.exp.sample(&mut self.rng);
        self.time += Time::new::<second>(delta_t);

        Some(self.time)
    }
}
