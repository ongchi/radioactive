mod reader;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use once_cell::sync::OnceCell;

use fixed_width_derive::FixedWidth;
use num_traits::FromPrimitive;
use serde::Deserialize;

use crate::error::Error;
use crate::primitive::attr::{AtomicMass, Energy, MassAttenuationCoefficient};
use crate::primitive::notation::Material;
use crate::primitive::Symbol;
use reader::{MassAttenCoefReader, MaterialConstantReader};

static MATERIAL_CONSTANTS: OnceCell<BTreeMap<Symbol, MaterialConstant>> = OnceCell::new();
static ATTENUATION_COEF: OnceCell<BTreeMap<Symbol, BTreeMap<Energy, MassAttenCoef>>> =
    OnceCell::new();

#[derive(Deserialize, FixedWidth)]
pub struct MaterialConstantRecord {
    #[fixed_width(range = "4..6")]
    symbol: Symbol,

    /// Z/A
    #[fixed_width(range = "30..40")]
    z_over_a: f64,

    /// I (eV)
    #[fixed_width(range = "40..50")]
    i: f64,

    /// Density (g/cm3)
    #[fixed_width(range = "50..60")]
    density: f64,
}

#[derive(Debug)]
pub struct MaterialConstant {
    /// Z/A
    pub z_over_a: f64,
    /// I (eV)
    pub i: f64,
    /// Density (g/cm3)
    pub density: f64,
}

impl From<MaterialConstantRecord> for MaterialConstant {
    fn from(record: MaterialConstantRecord) -> Self {
        Self {
            z_over_a: record.z_over_a,
            i: record.i,
            density: record.density,
        }
    }
}

#[derive(Deserialize, FixedWidth)]
pub struct MassAttenCoefRecord {
    /// Energy (MeV)
    #[fixed_width(range = "0..12")]
    energy: f64,

    /// mu/rho (cm2/g)
    #[fixed_width(range = "12..24")]
    mu_over_rho: f64,

    /// mu_en/rho (cm2/g)
    #[fixed_width(range = "24..36")]
    mu_en_over_rho: f64,
}

#[derive(Debug)]
pub struct MassAttenCoef {
    pub mu_over_rho: f64,
    pub mu_en_over_rho: f64,
}

impl From<MassAttenCoefRecord> for MassAttenCoef {
    fn from(record: MassAttenCoefRecord) -> Self {
        Self {
            mu_over_rho: record.mu_over_rho,
            mu_en_over_rho: record.mu_en_over_rho,
        }
    }
}

pub struct NistMassAttenCoef {
    path: PathBuf,
}

impl NistMassAttenCoef {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();

        if path.is_dir() {
            Ok(Self { path })
        } else {
            Err(Error::Unexpected(anyhow::anyhow!("Invalid data path")))
        }
    }

    pub fn material_constants(&self) -> Result<&BTreeMap<Symbol, MaterialConstant>, Error> {
        MATERIAL_CONSTANTS.get_or_try_init(|| {
            MaterialConstantReader::new(&self.path.join("material_constants"))?.read()
        })
    }

    pub fn mass_atten_coef(
        &self,
    ) -> Result<&BTreeMap<Symbol, BTreeMap<Energy, MassAttenCoef>>, Error> {
        ATTENUATION_COEF.get_or_try_init(|| {
            let mut content = BTreeMap::new();

            for z in 1..=92 {
                let symbol: Symbol = FromPrimitive::from_u8(z).unwrap();
                let value = MassAttenCoefReader::new(&self.path, z)?
                    .read()?
                    .into_iter()
                    .map(|r| ((r.energy * 1_000_000f64) as u32, r.into()))
                    .collect();

                content.insert(symbol, value);
            }

            Ok(content)
        })
    }
}

impl AtomicMass for NistMassAttenCoef {
    fn atomic_mass(&self, symbol: Symbol) -> Result<f64, Error> {
        self.material_constants()?
            .get(&symbol)
            .map(|r| ((symbol as u8) as f64) / r.z_over_a)
            .ok_or_else(|| Error::InvalidSymbol(symbol.to_string()))
    }
}

impl MassAttenuationCoefficient for NistMassAttenCoef {
    fn mass_attenuation_coefficient(
        &self,
        material: &Material,
        energy: Energy,
    ) -> Result<f64, Error> {
        let mut coef = 0f64;

        for (symbol, wf) in material.weight_fraction() {
            coef += wf
                * self
                    .mass_atten_coef()?
                    .get(symbol)
                    .unwrap()
                    .get(&energy)
                    .map(|r| r.mu_over_rho)
                    .ok_or(Error::InvalidEnergy(energy))?;
        }

        Ok(coef)
    }
}
