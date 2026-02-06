use std::collections::BTreeMap;

use super::dose_coefficient::{AgeGroup, DcfValue, Organ};
use super::notation::{Material, Symbol};
use super::nuclide::{HalfLife, Nuclide, Progeny};
use super::DecayModeSet;
use crate::error::Error;

/// Energy in eV
pub type Energy = u32;

pub trait NuclideProgeny {
    fn progeny(&self, nuclide: Nuclide) -> Result<Vec<Progeny>, Error>;
}

pub trait NuclideHalfLife {
    // Half life (arbitrary unit)
    fn half_life(&self, nuclide: Nuclide) -> Result<HalfLife, Error>;
}

pub trait NuclideDecayMode {
    fn decay_mode(&self, nuclide: Nuclide) -> Result<DecayModeSet, Error>;
}

pub trait DecayConstant {
    // Decay constant (s-1)
    fn lambda(&self, nuclide: Nuclide) -> Result<f64, Error>;
}

impl<T> DecayConstant for T
where
    T: NuclideHalfLife,
{
    fn lambda(&self, nuclide: Nuclide) -> Result<f64, Error> {
        self.half_life(nuclide).map(|hl| hl.as_lambda())
    }
}

pub trait Atom {
    fn symbol(&self) -> Symbol;
    fn nuclide(&self) -> Nuclide;
}

pub trait AtomicMass {
    /// Atomic mass (amu)
    fn atomic_mass(&self, symbol: Symbol) -> Result<f64, Error>;
}

pub trait MassAttenuationCoefficient {
    /// Mass attenuation coefficient (cm2/g)
    fn mass_attenuation_coefficient(
        &self,
        material: &Material,
        energy: Energy,
    ) -> Result<f64, Error>;
}

pub trait MeanFreePath {
    /// Mean free path (cm)
    fn mfp(&self, material: &Material, energy: Energy) -> Result<f64, Error>;
}

impl<T> MeanFreePath for T
where
    T: MassAttenuationCoefficient,
{
    fn mfp(&self, material: &Material, energy: Energy) -> Result<f64, Error> {
        self.mass_attenuation_coefficient(material, energy)
            .map(|mu_over_rho| (mu_over_rho * material.density()).recip())
    }
}

pub trait EffectiveAtomicNumber {
    // Effective atomic number
    fn z_eff(&self) -> Result<f64, Error>;
}

// Effective atomic number
pub fn z_eff(composition: &BTreeMap<Symbol, f64>) -> f64 {
    let weighted: BTreeMap<Symbol, f64> = composition
        .iter()
        .map(|(&symbol, &n)| (symbol, n * (symbol as u8) as f64))
        .collect();

    let tot_n: f64 = weighted.values().sum();

    let zeff: f64 = weighted
        .iter()
        .map(|(&symbol, &n)| {
            let z = (symbol as u8) as f64;
            (n / tot_n) * z.powf(2.94)
        })
        .sum();

    zeff.powf(2.94f64.recip())
}

/// Air submersion dose conversion factor
pub trait DcfAirSubmersion {
    fn dcf_air_submersion(&self, nuclide: Nuclide, organ: Organ)
        -> Result<Option<DcfValue>, Error>;
}

/// Water immersion dose conversion factor
pub trait DcfWaterImmersion {
    fn dcf_water_immersion(
        &self,
        nuclide: Nuclide,
        organ: Organ,
    ) -> Result<Option<DcfValue>, Error>;
}

/// Ground surface irradiation dose conversion factor
pub trait DcfGroundSurface {
    fn dcf_ground_surface(&self, nuclide: Nuclide, organ: Organ)
        -> Result<Option<DcfValue>, Error>;
}

/// Soil (1cm) irradiation dose conversion factor
pub trait DcfSoilOneCm {
    fn dcf_soil_1cm(&self, nuclide: Nuclide, organ: Organ) -> Result<Option<DcfValue>, Error>;
}

/// Soil (5cm) irradiation dose conversion factor
pub trait DcfSoilFiveCm {
    fn dcf_soil_5cm(&self, nuclide: Nuclide, organ: Organ) -> Result<Option<DcfValue>, Error>;
}

/// Soil (15cm) irradiation dose conversion factor
pub trait DcfSoilFifteenCm {
    fn dcf_soil_15cm(&self, nuclide: Nuclide, organ: Organ) -> Result<Option<DcfValue>, Error>;
}

/// Soil irradiation dose conversion factor
pub trait DcfSoilInfinite {
    fn dcf_soil_infinite(&self, nuclide: Nuclide, organ: Organ) -> Result<Option<DcfValue>, Error>;
}

pub trait DcfIngestion {
    fn dcf_ingestion(
        &self,
        nuclide: Nuclide,
        age_group: AgeGroup,
        organ: Organ,
    ) -> Result<Vec<DcfValue>, Error>;
}

pub trait DcfInhalation {
    fn dcf_inhalation(
        &self,
        nuclide: Nuclide,
        age_group: AgeGroup,
        organ: Organ,
    ) -> Result<Vec<DcfValue>, Error>;
}
