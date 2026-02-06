use std::path::Path;

use chumsky::prelude::Parser;
use mdbsql::Connection;

use crate::dataset::radtoolbox::utils::AsAgeDepPhantomOrgan;
use crate::error::Error;
use crate::primitive::attr::{DcfIngestion, DcfInhalation};
use crate::primitive::dose_coefficient::{
    AgeGroup, BiokineticAttr, DcfValue, Organ, RespiratoryTractAttr,
};
use crate::primitive::parser::gi_absorption_factor;
use crate::primitive::Nuclide;

#[derive(Debug)]
pub struct Icrp72 {
    connection: Connection,
}

impl Icrp72 {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            connection: Connection::open(path)?,
        })
    }
}

impl DcfIngestion for Icrp72 {
    fn dcf_ingestion(
        &self,
        nuclide: Nuclide,
        age_group: AgeGroup,
        organ: Organ,
    ) -> Result<Vec<DcfValue>, Error> {
        let rows = self.connection.prepare(&format!(
            "SELECT {}, f1 FROM \"Ingestion {}\" WHERE Nuclide='{}'",
            organ.to_col()?,
            age_group,
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
            res.push(DcfValue {
                value,
                unit,
                attr: Some(BiokineticAttr {
                    f1,
                    compound,
                    respiratory_tract_attr: None,
                }),
            })
        }

        Ok(res)
    }
}

impl DcfInhalation for Icrp72 {
    fn dcf_inhalation(
        &self,
        nuclide: Nuclide,
        age_group: AgeGroup,
        organ: Organ,
    ) -> Result<Vec<DcfValue>, Error> {
        let type_column = match age_group {
            // The column named `Tye` in table `Inhalation 5 yr-old` is a typo, obviously.
            AgeGroup::FiveYear => "Tye",
            _ => "Type",
        };
        let rows = self.connection.prepare(&format!(
            "SELECT {}, {}, f1 FROM \"Inhalation {}\" WHERE Nuclide='{}'",
            organ.to_col()?,
            type_column,
            age_group,
            nuclide
        ))?;

        let mut res = vec![];
        for row in rows {
            let value = row.get(0)?;
            let unit = "Sv/Bq".to_string();
            let respiratory_tract_attr = Some(RespiratoryTractAttr::ICRP66(row.get(1)?));
            let (f1, compound) = {
                let s = row.get::<String>(2)?;
                let r = gi_absorption_factor().parse(&s).into_result()?; r
            };
            res.push(DcfValue {
                value,
                unit,
                attr: Some(BiokineticAttr {
                    f1,
                    compound,
                    respiratory_tract_attr,
                }),
            })
        }

        Ok(res)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::primitive::dose_coefficient::PulmonaryAbsorptionType;

    const DATA_PATH: &str = "data/RadToolbox3/icrp72.mdb";

    #[test]
    #[ignore]
    fn ingestion_h3() {
        let db = Icrp72::open(DATA_PATH).unwrap();
        let results = db
            .dcf_ingestion(
                "H-3".parse().unwrap(),
                AgeGroup::Adult,
                Organ::EffectiveDose,
            )
            .unwrap();

        assert_eq!(
            results,
            vec![
                DcfValue {
                    value: 4.2e-11,
                    unit: "Sv/Bq".to_string(),
                    attr: Some(BiokineticAttr {
                        f1: 1.,
                        compound: "OBT".to_string(),
                        respiratory_tract_attr: None
                    })
                },
                DcfValue {
                    value: 1.8e-11,
                    unit: "Sv/Bq".to_string(),
                    attr: Some(BiokineticAttr {
                        f1: 1.,
                        compound: "".to_string(),
                        respiratory_tract_attr: None
                    })
                }
            ]
        );
    }

    #[test]
    #[ignore]
    fn inhalation_h3() {
        let db = Icrp72::open(DATA_PATH).unwrap();
        let results = db
            .dcf_inhalation(
                "H-3".parse().unwrap(),
                AgeGroup::Adult,
                Organ::EffectiveDose,
            )
            .unwrap();

        assert_eq!(
            results,
            vec![
                DcfValue {
                    value: 4.1e-11,
                    unit: "Sv/Bq".to_string(),
                    attr: Some(BiokineticAttr {
                        f1: 1.,
                        compound: "OBT".to_string(),
                        respiratory_tract_attr: Some(RespiratoryTractAttr::ICRP66(
                            PulmonaryAbsorptionType::Vapor
                        )),
                    })
                },
                DcfValue {
                    value: 1.8e-15,
                    unit: "Sv/Bq".to_string(),
                    attr: Some(BiokineticAttr {
                        f1: 1.,
                        compound: "HT".to_string(),
                        respiratory_tract_attr: Some(RespiratoryTractAttr::ICRP66(
                            PulmonaryAbsorptionType::Vapor
                        )),
                    })
                },
                DcfValue {
                    value: 1.8e-13,
                    unit: "Sv/Bq".to_string(),
                    attr: Some(BiokineticAttr {
                        f1: 1.,
                        compound: "CH3T".to_string(),
                        respiratory_tract_attr: Some(RespiratoryTractAttr::ICRP66(
                            PulmonaryAbsorptionType::Vapor
                        )),
                    })
                },
                DcfValue {
                    value: 1.8e-11,
                    unit: "Sv/Bq".to_string(),
                    attr: Some(BiokineticAttr {
                        f1: 1.,
                        compound: "HTO".to_string(),
                        respiratory_tract_attr: Some(RespiratoryTractAttr::ICRP66(
                            PulmonaryAbsorptionType::Vapor
                        )),
                    })
                },
                DcfValue {
                    value: 6.2e-12,
                    unit: "Sv/Bq".to_string(),
                    attr: Some(BiokineticAttr {
                        f1: 1.,
                        compound: "".to_string(),
                        respiratory_tract_attr: Some(RespiratoryTractAttr::ICRP66(
                            PulmonaryAbsorptionType::Fast
                        )),
                    })
                },
                DcfValue {
                    value: 4.5e-11,
                    unit: "Sv/Bq".to_string(),
                    attr: Some(BiokineticAttr {
                        f1: 0.1,
                        compound: "".to_string(),
                        respiratory_tract_attr: Some(RespiratoryTractAttr::ICRP66(
                            PulmonaryAbsorptionType::Moderate
                        )),
                    })
                },
                DcfValue {
                    value: 2.6e-10,
                    unit: "Sv/Bq".to_string(),
                    attr: Some(BiokineticAttr {
                        f1: 0.01,
                        compound: "".to_string(),
                        respiratory_tract_attr: Some(RespiratoryTractAttr::ICRP66(
                            PulmonaryAbsorptionType::Slow
                        )),
                    })
                },
            ]
        );
    }
}
