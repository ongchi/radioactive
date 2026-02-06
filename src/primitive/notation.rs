use std::collections::BTreeMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use chumsky::prelude::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::primitive::attr::AtomicMass;
use crate::primitive::parser::compound;

#[rustfmt::skip]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, FromPrimitive)]
pub enum Symbol {
    H = 1, He, Li, Be, B, C, N, O, F, Ne, Na, Mg, Al, Si, P, S, Cl, Ar, K, Ca,
    Sc, Ti, V, Cr, Mn, Fe, Co, Ni, Cu, Zn, Ga, Ge, As, Se, Br, Kr, Rb, Sr, Y,
    Zr, Nb, Mo, Tc, Ru, Rh, Pd, Ag, Cd, In, Sn, Sb, Te, I, Xe, Cs, Ba, La, Ce,
    Pr, Nd, Pm, Sm, Eu, Gd, Tb, Dy, Ho, Er, Tm, Yb, Lu, Hf, Ta, W, Re, Os, Ir,
    Pt, Au, Hg, Tl, Pb, Bi, Po, At, Rn, Fr, Ra, Ac, Th, Pa, U, Np, Pu, Am, Cm,
    Bk, Cf, Es, Fm, Md, No, Lr, Rf, Db, Sg, Bh, Hs, Mt, Ds, Rg, Cn, Nh, Fl, Mc,
    Lv, Ts, Og,
}

impl TryFrom<u8> for Symbol {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        FromPrimitive::from_u8(value).ok_or(Error::InvalidAtomicNumber(value))
    }
}

serde_plain::derive_fromstr_from_deserialize!(Symbol, |e| -> Error {
    Error::InvalidSymbol(e.to_string())
});
serde_plain::derive_display_from_serialize!(Symbol);

#[derive(Debug, PartialEq, Eq)]
pub enum Compound {
    Element(Symbol, u32),
    Molecule(Vec<Compound>, u32),
}

impl Display for Compound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Element(symbol, n) => {
                write!(f, "{}", symbol)?;
                if *n != 1 {
                    write!(f, "{}", n)?;
                }
            }
            Self::Molecule(g, mul) => {
                if *mul != 1 {
                    write!(f, "(")?;
                }
                for el in g {
                    el.fmt(f)?;
                }
                if *mul != 1 {
                    write!(f, "){}", mul)?;
                }
            }
        };
        Ok(())
    }
}

impl FromStr for Compound {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        compound().then_ignore(end()).parse(s).into_result().map_err(|e| e.into())
    }
}

impl Compound {
    pub fn composition(&self) -> BTreeMap<Symbol, u32> {
        let mut comp = BTreeMap::new();

        match self {
            Self::Element(symbol, n) => *comp.entry(*symbol).or_default() += n,
            Self::Molecule(g, mul) => {
                for el in g {
                    for (symbol, n) in &el.composition() {
                        *comp.entry(*symbol).or_default() += mul * n;
                    }
                }
            }
        }

        comp
    }
}

pub struct MaterialBuilder<D> {
    data: Arc<D>,
    composition: BTreeMap<Symbol, f64>,
    weight_fraction: BTreeMap<Symbol, f64>,
    density: Option<f64>,
    weight: Option<f64>,
}

impl<D> MaterialBuilder<D>
where
    D: AtomicMass,
{
    pub fn new(data: Arc<D>) -> Self {
        Self {
            data,
            composition: BTreeMap::new(),
            weight_fraction: BTreeMap::new(),
            density: None,
            weight: None,
        }
    }

    pub fn formula(mut self, formula: &str) -> Result<Self, Error> {
        let molecular = formula
            .parse::<Compound>()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let composition: BTreeMap<Symbol, f64> = molecular
            .composition()
            .into_iter()
            .map(|(k, v)| (k, v as f64))
            .collect();

        let mut weight_fraction = BTreeMap::new();
        for (&symbol, &n) in &composition {
            let mass = n * self.data.atomic_mass(symbol)?;
            weight_fraction.insert(symbol, mass);
        }

        let tot: f64 = weight_fraction.values().sum();
        for f in weight_fraction.values_mut() {
            *f /= tot;
        }

        self.composition = composition;
        self.weight_fraction = weight_fraction;

        Ok(self)
    }

    pub fn weights(mut self, weights: BTreeMap<Symbol, f64>) -> Result<Self, Error> {
        let mut weight_fraction = weights;
        let tot: f64 = weight_fraction.values().sum();

        for w in weight_fraction.values_mut() {
            *w /= tot;
        }

        self = self.weight_fraction(weight_fraction)?;
        self.weight = Some(tot);

        Ok(self)
    }

    pub fn weight_fraction(
        mut self,
        weight_fraction: BTreeMap<Symbol, f64>,
    ) -> Result<Self, Error> {
        let mut composition = BTreeMap::new();

        for (&symbol, &f) in &weight_fraction {
            let n = f / self.data.atomic_mass(symbol)?;
            composition.insert(symbol, n);
        }

        self.weight_fraction = weight_fraction;
        self.composition = composition;

        Ok(self)
    }

    pub fn density(mut self, density: f64) -> Self {
        self.density = Some(density);

        self
    }

    pub fn weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);

        self
    }

    pub fn build(self) -> Result<Material, Error> {
        match (self.weight, self.density) {
            (Some(weight), Some(density)) => Ok(Material {
                composition: self.composition,
                weight_fraction: self.weight_fraction,
                density,
                weight,
            }),
            _ => Err(anyhow::anyhow!("Data incomplete").into()),
        }
    }
}

#[derive(Debug)]
pub struct Material {
    composition: BTreeMap<Symbol, f64>,
    weight_fraction: BTreeMap<Symbol, f64>,
    density: f64,
    weight: f64,
}

impl Material {
    pub fn composition(&self) -> &BTreeMap<Symbol, f64> {
        &self.composition
    }

    pub fn weight_fraction(&self) -> &BTreeMap<Symbol, f64> {
        &self.weight_fraction
    }

    pub fn density(&self) -> f64 {
        self.density
    }

    pub fn weight(&self) -> f64 {
        self.weight
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct TestData;

    impl AtomicMass for TestData {
        fn atomic_mass(&self, symbol: Symbol) -> Result<f64, Error> {
            if symbol == Symbol::H {
                Ok(1.)
            } else if symbol == Symbol::O {
                Ok(3.)
            } else {
                Err(Error::InvalidSymbol(symbol.to_string()))
            }
        }
    }

    #[test]
    fn molecular() {
        let ether: Compound = "(C2H5)2O".parse().unwrap();

        assert_eq!(format!("{}", ether), "(C2H5)2O");
        assert_eq!(ether.composition().get(&Symbol::H), Some(&10));
        assert_eq!(ether.composition().get(&Symbol::C), Some(&4));
        assert_eq!(ether.composition().get(&Symbol::O), Some(&1));
    }

    #[test]
    fn material() {
        let data = Arc::new(TestData {});
        let material = MaterialBuilder::new(data)
            .formula("HO")
            .unwrap()
            .weight(1.)
            .density(1.)
            .build()
            .unwrap();

        assert_eq!(material.weight(), 1.);
        assert_eq!(material.density(), 1.);
        assert_eq!(material.composition().get(&Symbol::H), Some(&1.));
        assert_eq!(material.composition().get(&Symbol::O), Some(&1.));
        assert_eq!(material.weight_fraction().get(&Symbol::H), Some(&0.25));
        assert_eq!(material.weight_fraction().get(&Symbol::O), Some(&0.75));
    }
}
