use std::path::Path;

use chumsky::prelude::Parser;
use mdbsql::Connection;

use crate::dataset::radtoolbox::utils::AsAdultPhantomOrgan;
use crate::error::Error;
use crate::primitive::dose_coefficient::{
    AgeGroup, BiokineticAttr, DcfValue, Organ, RespiratoryTractAttr,
};
use crate::primitive::parser::gi_absorption_factor;
use crate::primitive::{
    DcfAirSubmersion, DcfGroundSurface, DcfIngestion, DcfInhalation, DcfSoilFifteenCm,
    DcfSoilFiveCm, DcfSoilInfinite, DcfSoilOneCm, DcfWaterImmersion, Nuclide,
};

#[derive(Debug)]
pub struct Fgr12 {
    connection: Connection,
}

impl Fgr12 {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            connection: Connection::open(path.as_ref())?,
        })
    }
}

macro_rules! ext_dcf_fn {
    ($fn:ident, $table:expr, $unit:expr) => {
        fn $fn(&self, nuclide: Nuclide, organ: Organ) -> Result<Option<DcfValue>, Error> {
            if let Some(row) = self
                .connection
                .prepare(&format!(
                    concat!("SELECT \"{}\" FROM \"", $table, "\" WHERE Nuclide='{}'"),
                    organ.to_col()?,
                    nuclide
                ))?
                .next()
            {
                Ok(Some(DcfValue {
                    value: row.get(0)?,
                    unit: $unit.to_string(),
                    attr: None,
                }))
            } else {
                Ok(None)
            }
        }
    };
}

impl DcfAirSubmersion for Fgr12 {
    ext_dcf_fn!(dcf_air_submersion, "Air Submersion", "Sv/hr per Bq/m3");
}

impl DcfWaterImmersion for Fgr12 {
    ext_dcf_fn!(dcf_water_immersion, "Water Submersion", "Sv/hr per Bq/m3");
}

impl DcfGroundSurface for Fgr12 {
    ext_dcf_fn!(dcf_ground_surface, "Ground Surface", "Sv/hr per Bq/m2");
}

impl DcfSoilOneCm for Fgr12 {
    ext_dcf_fn!(dcf_soil_1cm, "1 cm Soil", "Sv/hr per Bq/m3");
}

impl DcfSoilFiveCm for Fgr12 {
    ext_dcf_fn!(dcf_soil_5cm, "5 cm Soil", "Sv/hr per Bq/m3");
}

impl DcfSoilFifteenCm for Fgr12 {
    ext_dcf_fn!(dcf_soil_15cm, "15 cm Soil", "Sv/hr per Bq/m3");
}

impl DcfSoilInfinite for Fgr12 {
    ext_dcf_fn!(dcf_soil_infinite, "Infinite Soil", "Sv/hr per Bq/m3");
}

impl DcfIngestion for Fgr12 {
    fn dcf_ingestion(
        &self,
        nuclide: Nuclide,
        age_group: AgeGroup,
        organ: Organ,
    ) -> Result<Vec<DcfValue>, Error> {
        match age_group {
            AgeGroup::Worker => {
                let rows = self.connection.prepare(&format!(
                    "SELECT \"{}\", f1 FROM Ingestion WHERE Nuclide='{}'",
                    organ.to_col()?,
                    nuclide
                ))?;

                let mut res = vec![];
                for row in rows {
                    let value = row.get(0)?;
                    let unit = "Sv/Bq".to_string();
                    let (f1, compound) = {
                        let s = row.get::<String>(1)?;
                        let r = gi_absorption_factor().parse(&s).into_result()?; r
                    };
                    let attr = Some(BiokineticAttr {
                        compound,
                        f1,
                        respiratory_tract_attr: None,
                    });

                    res.push(DcfValue { value, unit, attr })
                }

                Ok(res)
            }
            _ => Ok(vec![]),
        }
    }
}

impl DcfInhalation for Fgr12 {
    fn dcf_inhalation(
        &self,
        nuclide: Nuclide,
        age_group: AgeGroup,
        organ: Organ,
    ) -> Result<Vec<DcfValue>, Error> {
        match age_group {
            AgeGroup::Worker => {
                let rows = self.connection.prepare(&format!(
                    "SELECT \"{}\", Class, f1 FROM Inhalation WHERE Nuclide='{}'",
                    organ.to_col()?,
                    nuclide
                ))?;

                let mut res = vec![];
                for row in rows {
                    let value = row.get(0)?;
                    let unit = "Sv/Bq".to_string();
                    let respiratory_tract_attr = Some(RespiratoryTractAttr::ICRP30(row.get(1)?));
                    let (f1, compound) = {
                        let s = row.get::<String>(2)?;
                        let r = gi_absorption_factor().parse(&s).into_result()?; r
                    };
                    let attr = Some(BiokineticAttr {
                        compound,
                        f1,
                        respiratory_tract_attr,
                    });
                    res.push(DcfValue { value, unit, attr })
                }

                Ok(res)
            }
            _ => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::primitive::dose_coefficient::ClearanceClass;

    const DATA_PATH: &str = "data/RadToolbox3/fgr12.mdb";

    #[test]
    #[ignore]
    fn air_submersion_cs137() {
        let db = Fgr12::open(DATA_PATH).unwrap();
        let result = db
            .dcf_air_submersion("Cs-137".parse().unwrap(), Organ::EffectiveDose)
            .unwrap();

        assert_eq!(result.unwrap().value, 9.28e-17);
    }

    #[test]
    #[ignore]
    fn water_submersion_cs137() {
        let db = Fgr12::open(DATA_PATH).unwrap();
        let result = db
            .dcf_water_immersion("Cs-137".parse().unwrap(), Organ::EffectiveDose)
            .unwrap();

        assert_eq!(result.unwrap().value, 1.04e-19);
    }

    #[test]
    #[ignore]
    fn ground_surface_cs137() {
        let db = Fgr12::open(DATA_PATH).unwrap();
        let result = db
            .dcf_ground_surface("Cs-137".parse().unwrap(), Organ::EffectiveDose)
            .unwrap();

        assert_eq!(result.unwrap().value, 2.99e-18);
    }

    #[test]
    #[ignore]
    fn soil_1cm_cs137() {
        let db = Fgr12::open(DATA_PATH).unwrap();
        let result = db
            .dcf_soil_1cm("Cs-137".parse().unwrap(), Organ::EffectiveDose)
            .unwrap();

        assert_eq!(result.unwrap().value, 2.07e-21);
    }

    #[test]
    #[ignore]
    fn soil_5cm_cs137() {
        let db = Fgr12::open(DATA_PATH).unwrap();
        let result = db
            .dcf_soil_5cm("Cs-137".parse().unwrap(), Organ::EffectiveDose)
            .unwrap();

        assert_eq!(result.unwrap().value, 3.62e-21);
    }

    #[test]
    #[ignore]
    fn soil_15cm_cs137() {
        let db = Fgr12::open(DATA_PATH).unwrap();
        let result = db
            .dcf_soil_15cm("Cs-137".parse().unwrap(), Organ::EffectiveDose)
            .unwrap();

        assert_eq!(result.unwrap().value, 4.4e-21);
    }

    #[test]
    #[ignore]
    fn soil_infinite_cs137() {
        let db = Fgr12::open(DATA_PATH).unwrap();
        let result = db
            .dcf_soil_infinite("Cs-137".parse().unwrap(), Organ::EffectiveDose)
            .unwrap();

        assert_eq!(result.unwrap().value, 4.47e-21);
    }

    #[test]
    #[ignore]
    fn ingestion_h3() {
        let db = Fgr12::open(DATA_PATH).unwrap();
        let results = db
            .dcf_ingestion(
                "H-3".parse().unwrap(),
                AgeGroup::Worker,
                Organ::EffectiveDoseEquivalent,
            )
            .unwrap();

        assert_eq!(
            results,
            vec![DcfValue {
                value: 1.73e-11,
                unit: "Sv/Bq".to_string(),
                attr: Some(BiokineticAttr {
                    compound: "".to_string(),
                    f1: 1.,
                    respiratory_tract_attr: None
                }),
            }]
        );
    }

    #[test]
    #[ignore]
    fn inhalation_h3() {
        let db = Fgr12::open(DATA_PATH).unwrap();
        let results = db
            .dcf_inhalation(
                "H-3".parse().unwrap(),
                AgeGroup::Worker,
                Organ::EffectiveDoseEquivalent,
            )
            .unwrap();

        assert_eq!(
            results,
            vec![DcfValue {
                value: 1.73e-11,
                unit: "Sv/Bq".to_string(),
                attr: Some(BiokineticAttr {
                    compound: "".to_string(),
                    f1: 1.,
                    respiratory_tract_attr: Some(RespiratoryTractAttr::ICRP30(
                        ClearanceClass::WaterVapor
                    )),
                }),
            },]
        );
    }
}
