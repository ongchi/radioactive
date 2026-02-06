mod graph;

pub use graph::{DecayChain, DecayChainBuilder};

use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use crate::primitive::attr::{DecayConstant, NuclideProgeny};
use crate::primitive::Nuclide;

#[derive(Debug, Clone)]
pub struct Inventory(BTreeMap<Nuclide, f64>);

impl Deref for Inventory {
    type Target = BTreeMap<Nuclide, f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Inventory {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn add(&mut self, nuclide: Nuclide, activity: f64) {
        let inner = &mut self.0;
        *inner.entry(nuclide).or_insert(0.0) += activity;
    }

    pub fn remove(&mut self, nuclide: Nuclide) -> Option<(Nuclide, f64)> {
        self.0.remove_entry(&nuclide)
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

type CachedNode = BTreeMap<Nuclide, Vec<(Vec<f64>, Vec<f64>)>>;
type CachedData = BTreeMap<Nuclide, Arc<CachedNode>>;

#[derive(Debug)]
pub struct BatemanDecaySolver<D> {
    decay_data: Arc<D>,
    pub cache: RwLock<CachedData>,
}

impl<D> BatemanDecaySolver<D>
where
    D: NuclideProgeny + DecayConstant,
{
    pub fn new(decay_data: Arc<D>) -> Arc<Self> {
        Arc::new(Self {
            decay_data,
            cache: RwLock::new(BTreeMap::new()),
        })
    }

    /// Decay calculation for decay_time in seconds.
    pub fn decay(&self, inventory: &Inventory, decay_time: f64) -> Inventory {
        let mut inv = Inventory::new();

        for (&nuclide, &activity) in inventory.iter() {
            if let Some(bateman_res) = self.bateman_eq(nuclide, decay_time) {
                for (nuc, res) in bateman_res {
                    inv.add(nuc, activity * res);
                }
            }
        }

        inv
    }

    // Bateman Equation
    pub fn bateman_eq(&self, nuclide: Nuclide, dt: f64) -> Option<BTreeMap<Nuclide, f64>> {
        self.cached_vars(nuclide).map(|cache| {
            let mut res = BTreeMap::new();
            for (&nuc, vars) in cache.iter() {
                for (br, lamb) in vars {
                    *res.entry(nuc).or_insert(0.) += lamb[1..].iter().product::<f64>()
                        * br.iter().product::<f64>()
                        * (lamb.iter().enumerate())
                            .map(|(i, &li)| {
                                (-li * dt).exp()
                                    / (lamb.iter().enumerate().filter(|(j, _)| i != *j))
                                        .map(|(_, &lj)| lj - li)
                                        .product::<f64>()
                            })
                            .sum::<f64>();
                }
            }
            res
        })
    }

    // Variables for calculate with Bateman Equation
    fn cached_vars(&self, parent: Nuclide) -> Option<Arc<CachedNode>> {
        let cache = self.cache.read().unwrap();

        if let Some(brs_lambs) = cache.get(&parent) {
            Some(brs_lambs.clone())
        } else {
            drop(cache);
            let mut cache = self.cache.write().unwrap();

            let mut stack = vec![(parent, vec![], vec![self.decay_data.lambda(parent).ok()?])];
            let mut brs_lambs: CachedNode = BTreeMap::new();

            while let Some((parent, br, lambda)) = stack.pop() {
                brs_lambs
                    .entry(parent)
                    .or_default()
                    .push((br.clone(), lambda.clone()));

                for daughter in self.decay_data.progeny(parent).unwrap() {
                    if let Ok(lambda_d) = self.decay_data.lambda(daughter.nuclide) {
                        let mut br = br.clone();
                        br.push(daughter.branch_rate);
                        let mut lambda = lambda.clone();
                        lambda.push(lambda_d);
                        stack.push((daughter.nuclide, br, lambda));
                    }
                }
            }

            let brs_lambs = Arc::new(brs_lambs);
            cache.insert(parent, brs_lambs.clone());

            Some(brs_lambs)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::{
        error::Error,
        primitive::{DecayModeSet, Progeny},
    };

    struct TestData {
        progeny: BTreeMap<Nuclide, Vec<Progeny>>,
    }

    impl TestData {
        fn new() -> Arc<Self> {
            let mut progeny = BTreeMap::new();

            macro_rules! insert_progeny {
                ($n:expr, $d:expr, $br:expr) => {
                    progeny.insert(
                        $n.parse().unwrap(),
                        vec![Progeny {
                            nuclide: $d.parse().unwrap(),
                            branch_rate: $br,
                            decay_mode: DecayModeSet::default(),
                        }],
                    )
                };

                ($n:expr) => {
                    progeny.insert($n.parse().unwrap(), vec![])
                };
            }

            insert_progeny!("Nb-99", "Mo-99", 0.7);
            insert_progeny!("Mo-99", "Tc-99m", 0.3);
            insert_progeny!("Tc-99m");

            Arc::new(Self { progeny })
        }
    }

    impl NuclideProgeny for TestData {
        fn progeny(&self, nuclide: Nuclide) -> Result<Vec<Progeny>, Error> {
            self.progeny
                .get(&nuclide)
                .cloned()
                .ok_or(Error::InvalidNuclide(nuclide.to_string()))
        }
    }

    impl DecayConstant for TestData {
        fn lambda(&self, nuclide: Nuclide) -> Result<f64, Error> {
            if nuclide == "Nb-99".parse().unwrap() {
                Ok(2.0_f64.ln())
            } else if nuclide == "Mo-99".parse().unwrap() {
                Ok(2.0_f64.ln() / 2.)
            } else if nuclide == "Tc-99m".parse().unwrap() {
                Ok(2.0_f64.ln() / 4.)
            } else {
                Err(Error::InvalidNuclide(nuclide.to_string()))
            }
        }
    }

    #[test]
    fn bateman_solver() {
        let data = TestData::new();
        let solver = BatemanDecaySolver::new(data);

        let mut inv = Inventory::new();
        inv.add("Nb-99".parse().unwrap(), 1.0);

        let res = solver.decay(&inv, 1.0);

        let l1 = 2.0_f64.ln();
        let l2 = 2.0_f64.ln() / 2.;
        let l3 = 2.0_f64.ln() / 4.;

        let br1 = 0.7;
        let br2 = 0.3;

        assert_eq!(res.get(&"Nb-99".parse().unwrap()), Some(&((-l1).exp())));
        assert_eq!(
            res.get(&"Mo-99".parse().unwrap()),
            Some(&(l2 * br1 * ((-l1).exp() / (l2 - l1) + (-l2).exp() / (l1 - l2))))
        );
        assert_eq!(
            res.get(&"Tc-99m".parse().unwrap()),
            Some(
                &((l2 * l3)
                    * (br1 * br2)
                    * ((-l1).exp() / ((l2 - l1) * (l3 - l1))
                        + (-l2).exp() / ((l1 - l2) * (l3 - l2))
                        + (-l3).exp() / ((l1 - l3) * (l2 - l3))))
            )
        );
    }
}
