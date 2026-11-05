mod reader;

use std::collections::BTreeMap;
use std::path::Path;

use fixed_width_derive::FixedWidth;
use serde::Deserialize;

use crate::error::Error;
use crate::primitive::attr::{AtomicMass, Energy, MassAttenuationCoefficient};
use crate::primitive::notation::Material;
use crate::primitive::Symbol;
use reader::{read_mass_atten_coef, read_material_constants};

/// Number of elements in the tables (Z = 1 to 92)
pub const ELEMENT_COUNT: usize = 92;

/// Relative distance within which an energy is a table energy
const TABLE_ENERGY_TOLERANCE: f64 = 1e-12;

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

#[derive(Debug, PartialEq)]
pub struct MassAttenCoef {
    /// Energy (MeV)
    pub energy: f64,
    pub mu_over_rho: f64,
    pub mu_en_over_rho: f64,
}

impl From<MassAttenCoefRecord> for MassAttenCoef {
    fn from(record: MassAttenCoefRecord) -> Self {
        Self {
            energy: record.energy,
            mu_over_rho: record.mu_over_rho,
            mu_en_over_rho: record.mu_en_over_rho,
        }
    }
}

/// NIST X-ray mass attenuation coefficients (Hubbell and Seltzer) of the elements
/// Z = 1 to 92
pub struct NistMassAttenCoef {
    material_constants: BTreeMap<Symbol, MaterialConstant>,
    mass_atten_coef: BTreeMap<Symbol, Vec<MassAttenCoef>>,
}

impl NistMassAttenCoef {
    /// Read the tables of a folder: `material_constants` and one file for each
    /// element (`01` to `92`), as `data/download.py` writes them
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref();

        if !path.is_dir() {
            return Err(Error::Unexpected(anyhow::anyhow!("Invalid data path")));
        }

        let material_constants = std::fs::read_to_string(path.join("material_constants"))?;
        let element_tables = (1..=ELEMENT_COUNT)
            .map(|z| std::fs::read_to_string(path.join(format!("{:02}", z))))
            .collect::<Result<Vec<_>, _>>()?;

        Self::from_texts(
            &material_constants,
            &element_tables
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )
    }

    /// Build from the text of `material_constants` and of the element tables
    /// (Z = 1, 2, ...)
    pub fn from_texts(material_constants: &str, element_tables: &[&str]) -> Result<Self, Error> {
        if element_tables.len() != ELEMENT_COUNT {
            return Err(Error::Unexpected(anyhow::anyhow!(
                "{} element tables, expected {}",
                element_tables.len(),
                ELEMENT_COUNT
            )));
        }

        let mass_atten_coef = element_tables
            .iter()
            .zip(1u8..)
            .map(|(text, z)| Ok((Symbol::try_from(z)?, read_mass_atten_coef(text)?)))
            .collect::<Result<_, Error>>()?;

        Ok(Self {
            material_constants: read_material_constants(material_constants)?,
            mass_atten_coef,
        })
    }

    pub fn material_constants(&self) -> &BTreeMap<Symbol, MaterialConstant> {
        &self.material_constants
    }

    /// Rows of each element in the order of the table; an absorption edge has two
    /// rows with the same energy, below and above the edge
    pub fn mass_atten_coef(&self) -> &BTreeMap<Symbol, Vec<MassAttenCoef>> {
        &self.mass_atten_coef
    }

    fn element_rows(&self, symbol: Symbol) -> Result<&[MassAttenCoef], Error> {
        self.mass_atten_coef
            .get(&symbol)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::InvalidSymbol(symbol.to_string()))
    }

    /// Mass attenuation coefficient (cm2/g) of an element at any energy (eV) of the
    /// table range, by log-log interpolation between the table energies. At the
    /// energy of an absorption edge, the value above the edge.
    pub fn element_mass_attenuation_coefficient_at(
        &self,
        symbol: Symbol,
        energy: f64,
    ) -> Result<f64, Error> {
        let rows = self.element_rows(symbol)?;
        let energy_mev = energy / 1_000_000f64;
        // Energies within this distance of a table energy are that energy: the
        // conversion from eV can be one rounding step off
        let tolerance = TABLE_ENERGY_TOLERANCE * energy_mev;

        let is_in_range = rows.first().zip(rows.last()).is_some_and(|(first, last)| {
            first.energy - tolerance <= energy_mev && energy_mev <= last.energy + tolerance
        });
        if !is_in_range {
            return Err(Error::EnergyOutOfRange(energy));
        }

        // The last row at or below the energy: the upper row of an edge at its energy
        let lower = rows.partition_point(|row| row.energy <= energy_mev + tolerance) - 1;
        let lower_row = &rows[lower];
        if (lower_row.energy - energy_mev).abs() <= tolerance {
            return Ok(lower_row.mu_over_rho);
        }
        let upper_row = &rows[lower + 1];

        let fraction =
            (energy_mev / lower_row.energy).ln() / (upper_row.energy / lower_row.energy).ln();
        Ok(lower_row.mu_over_rho * (upper_row.mu_over_rho / lower_row.mu_over_rho).powf(fraction))
    }

    /// Mass attenuation coefficient (cm2/g) of a material at any energy (eV) of the
    /// table range: the sum of the element values by weight fraction
    pub fn mass_attenuation_coefficient_at(
        &self,
        material: &Material,
        energy: f64,
    ) -> Result<f64, Error> {
        material
            .weight_fraction()
            .iter()
            .map(|(&symbol, wf)| {
                Ok(wf * self.element_mass_attenuation_coefficient_at(symbol, energy)?)
            })
            .sum()
    }
}

impl AtomicMass for NistMassAttenCoef {
    fn atomic_mass(&self, symbol: Symbol) -> Result<f64, Error> {
        self.material_constants
            .get(&symbol)
            .map(|r| ((symbol as u8) as f64) / r.z_over_a)
            .ok_or_else(|| Error::InvalidSymbol(symbol.to_string()))
    }
}

impl MassAttenuationCoefficient for NistMassAttenCoef {
    /// Value at a table energy only; at an absorption edge, the value above the edge
    fn mass_attenuation_coefficient(
        &self,
        material: &Material,
        energy: Energy,
    ) -> Result<f64, Error> {
        let mut coef = 0f64;

        for (&symbol, wf) in material.weight_fraction() {
            coef += wf
                * self
                    .element_rows(symbol)?
                    .iter()
                    .rev()
                    .find(|r| (r.energy * 1_000_000f64) as u32 == energy)
                    .map(|r| r.mu_over_rho)
                    .ok_or(Error::InvalidEnergy(energy))?;
        }

        Ok(coef)
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::*;
    use crate::primitive::notation::MaterialBuilder;

    const DATA_PATH: &str = "data/XrayMassAttenCoef";

    fn data() -> Arc<NistMassAttenCoef> {
        Arc::new(NistMassAttenCoef::open(DATA_PATH).unwrap())
    }

    fn is_close(value: f64, expected: f64, relative_tolerance: f64) -> bool {
        (value - expected).abs() <= relative_tolerance * expected.abs()
    }

    #[test]
    #[ignore]
    fn open_reads_all_elements() {
        let data = data();

        assert_eq!(data.mass_atten_coef().len(), ELEMENT_COUNT);
        assert_eq!(data.material_constants().len(), ELEMENT_COUNT);
        assert_eq!(data.material_constants()[&Symbol::U].density, 18.95);
    }

    #[test]
    fn from_texts_needs_all_element_tables() {
        let header = "Energy      mu/rho      mu_en/rho\n(MeV)       (cm2/g)     (cm2/g)\n";

        assert!(NistMassAttenCoef::from_texts(header, &[header]).is_err());
    }

    #[test]
    #[ignore]
    fn interpolation_is_exact_at_table_energies() {
        let data = data();

        for (&symbol, rows) in data.mass_atten_coef() {
            for (index, row) in rows.iter().enumerate() {
                let is_lower_edge_row = rows
                    .get(index + 1)
                    .is_some_and(|next| next.energy == row.energy);
                if is_lower_edge_row {
                    continue;
                }
                let value = data
                    .element_mass_attenuation_coefficient_at(symbol, row.energy * 1_000_000f64)
                    .unwrap();
                assert!(
                    is_close(value, row.mu_over_rho, 1e-12),
                    "{symbol} at {} MeV",
                    row.energy
                );
            }
        }
    }

    #[test]
    #[ignore]
    fn interpolation_keeps_both_sides_of_the_lead_k_edge() {
        let data = data();
        let edge = 88_004.5;

        let below = data
            .element_mass_attenuation_coefficient_at(Symbol::Pb, edge - 1e-3)
            .unwrap();
        let above = data
            .element_mass_attenuation_coefficient_at(Symbol::Pb, edge)
            .unwrap();

        assert!(is_close(below, 1.910, 1e-6), "{below}");
        assert_eq!(above, 7.683);
    }

    #[test]
    #[ignore]
    fn interpolation_is_log_log_between_table_energies() {
        let data = data();

        // H: 1.00000E-01 2.944E-01 and 1.50000E-01 2.651E-01
        let value = data
            .element_mass_attenuation_coefficient_at(Symbol::H, 120_000.0)
            .unwrap();
        let fraction = (1.2f64).ln() / (1.5f64).ln();
        let expected = 2.944e-1 * (2.651e-1f64 / 2.944e-1).powf(fraction);

        assert!(is_close(value, expected, 1e-12), "{value} {expected}");
    }

    #[test]
    #[ignore]
    fn energy_outside_the_table_is_an_error() {
        let data = data();

        for energy in [999.0, 2.0e7 + 1.0] {
            assert!(matches!(
                data.element_mass_attenuation_coefficient_at(Symbol::H, energy),
                Err(Error::EnergyOutOfRange(_))
            ));
        }
    }

    #[test]
    #[ignore]
    fn element_beyond_the_tables_is_an_error() {
        assert!(matches!(
            data().element_mass_attenuation_coefficient_at(Symbol::Np, 50_000.0),
            Err(Error::InvalidSymbol(_))
        ));
    }

    #[test]
    #[ignore]
    fn water_mixture_matches_the_nist_water_table() {
        let data = data();
        let water = MaterialBuilder::new(data.clone())
            .formula("H2O")
            .unwrap()
            .weight(1.)
            .density(1.)
            .build()
            .unwrap();

        // NIST Table 4, water (liquid): energy (MeV), mu/rho (cm2/g)
        let nist_water = [
            (1.0e-2, 5.329),
            (1.5e-2, 1.673),
            (2.0e-2, 8.096e-1),
            (3.0e-2, 3.756e-1),
            (4.0e-2, 2.683e-1),
            (5.0e-2, 2.269e-1),
            (6.0e-2, 2.059e-1),
            (8.0e-2, 1.837e-1),
            (1.0e-1, 1.707e-1),
            (1.5e-1, 1.505e-1),
        ];

        for (energy, expected) in nist_water {
            let value = data
                .mass_attenuation_coefficient_at(&water, energy * 1_000_000f64)
                .unwrap();
            assert!(
                is_close(value, expected, 0.01),
                "{energy} MeV: {value} {expected}"
            );
        }
    }
}
