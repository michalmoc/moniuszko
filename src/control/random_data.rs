use rand::{Rng, SeedableRng, random_range};
use rand_pcg::Pcg32;
use std::mem;

enum RandomMode {
    Pcg {
        rng: Pcg32,
    },
    Sattolo {
        permutation: Vec<u32>,
        rev_permutation: Vec<u32>,
    },
}

pub struct RandomData {
    n_items: u32,
    mode: RandomMode,
}

impl RandomData {
    pub fn pcg(n_items: u32) -> RandomData {
        RandomData {
            n_items,
            mode: RandomMode::Pcg {
                rng: Pcg32::from_rng(&mut rand::rng()),
            },
        }
    }

    pub fn sattolo(n_items: u32) -> RandomData {
        let mut permutation = Vec::from_iter(0..n_items);
        let mut rev_permutation = Vec::from_iter(0..n_items);

        let mut i = n_items as usize;
        while i > 1 {
            i -= 1;
            let j = random_range(0..i);
            let [pi, pj] = permutation.get_disjoint_mut([i, j]).unwrap();
            mem::swap(pi, pj);
        }

        for i in 0..n_items as usize {
            rev_permutation[permutation[i] as usize] = i as u32;
        }

        RandomData {
            n_items,
            mode: RandomMode::Sattolo {
                permutation,
                rev_permutation,
            },
        }
    }

    pub fn next(&mut self, current: u32) -> u32 {
        match &mut self.mode {
            RandomMode::Pcg { rng } => rng.next_u32() % self.n_items,
            RandomMode::Sattolo { permutation, .. } => permutation[current as usize],
        }
    }

    pub fn prev(&mut self, current: u32) -> u32 {
        match &mut self.mode {
            RandomMode::Pcg { rng } => {
                rng.advance(-2_i64 as u64);
                rng.next_u32() % self.n_items
            }
            RandomMode::Sattolo {
                rev_permutation, ..
            } => rev_permutation[current as usize],
        }
    }
}
