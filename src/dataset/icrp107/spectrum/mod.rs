pub(super) mod ack;
pub(super) mod bet;
pub(super) mod nsf;
pub(super) mod rad;

use serde::Deserialize;

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
pub enum RadiationType {
    #[serde(rename = "G")]
    Gamma,
    #[serde(rename = "PG")]
    PromptGamma,
    #[serde(rename = "DG")]
    DelayedGamma,
    X,
    #[serde(rename = "AQ")]
    AnnihilationPhoton,
    #[serde(rename = "B+")]
    BetaPlus,
    #[serde(rename = "B-")]
    BetaMinus,
    #[serde(rename = "DB")]
    DelayedBeta,
    #[serde(rename = "IE")]
    InternalConversionElectron,
    #[serde(rename = "AE")]
    AugerElectron,
    #[serde(rename = "A")]
    Alpha,
    #[serde(rename = "AR")]
    AlphaRecoil,
    #[serde(rename = "FF")]
    FissionFragment,
    #[serde(rename = "N")]
    NeutronEmission,
}

impl RadiationType {
    pub fn is_photon(self) -> bool {
        matches!(
            self,
            Self::Gamma
                | Self::PromptGamma
                | Self::DelayedGamma
                | Self::X
                | Self::AnnihilationPhoton
        )
    }
}

#[derive(Debug)]
pub enum Spectrum {
    Radiation {
        r#type: RadiationType,
        r#yield: f64,
        energy: f64,
    },
    Beta {
        energy: f64,
        number: f64,
    },
    AugerCosterKronigElectron {
        r#yield: f64,
        energy: f64,
        transition: String,
    },
    SpontaneousFissionNeutron {
        energy_lower: f64,
        energy_upper: f64,
        r#yield: f64,
    },
}
