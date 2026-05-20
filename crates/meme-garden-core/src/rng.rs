use rand::{Rng, RngCore, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// Deterministic RNG for the simulation. All stochastic decisions in `core` MUST
/// go through this wrapper so a given seed produces a bit-identical run.
#[derive(Debug, Clone)]
pub struct SimRng {
    inner: Pcg64Mcg,
}

impl SimRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: Pcg64Mcg::seed_from_u64(seed),
        }
    }

    pub fn gen_bool(&mut self, p: f32) -> bool {
        if p <= 0.0 {
            return false;
        }
        if p >= 1.0 {
            return true;
        }
        self.inner.gen::<f32>() < p
    }

    pub fn gen_range_usize(&mut self, lo: usize, hi: usize) -> usize {
        self.inner.gen_range(lo..hi)
    }

    pub fn gen_u32(&mut self) -> u32 {
        self.inner.next_u32()
    }
}

impl RngCore for SimRng {
    fn next_u32(&mut self) -> u32 {
        self.inner.next_u32()
    }
    fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.inner.fill_bytes(dest)
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.inner.try_fill_bytes(dest)
    }
}
