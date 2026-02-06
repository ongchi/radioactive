use std::fmt::Display;
use std::str::FromStr;

use chumsky::prelude::*;
use float_pretty_print::PrettyPrintFloat;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_with::DeserializeFromStr;

use super::notation::Symbol;
use super::parser::{halflife, nuclide};
use crate::error::Error;

pub use decay_mode::{DecayMode, DecayModeSet};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, DeserializeFromStr)]
pub enum Nuclide {
    /// Nuclide with canonical id
    WithId(u32),
    /// Spontaneous fission products
    FissionProducts,
}

impl Nuclide {
    pub fn id(&self) -> Option<u32> {
        match self {
            Self::WithId(id) => Some(*id),
            Self::FissionProducts => None,
        }
    }

    // Atomic number
    pub fn z(&self) -> Option<u8> {
        self.id().map(|id| (id / 10_000_000) as u8)
    }

    // Mass number
    pub fn a(&self) -> Option<u32> {
        self.id().map(|id| (id / 10_000) % 1_000)
    }

    pub fn state(&self) -> Option<MetastableState> {
        self.id()
            .and_then(|id| FromPrimitive::from_u8((id % 10) as u8))
    }
}

impl Display for Nuclide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WithId(_) => {
                let symbol = Symbol::try_from(self.z().unwrap()).unwrap();
                write!(f, "{}-{}", symbol, self.a().unwrap())?;
                if let Some(state) = self.state() {
                    write!(f, "{}", state)?;
                }
                Ok(())
            }
            Self::FissionProducts => write!(f, "various"),
        }
    }
}

impl FromStr for Nuclide {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        nuclide()
            .then_ignore(end())
            .parse(s.trim())
            .into_result()
            .map_err(|_| Error::InvalidNuclide(s.to_string()))
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, FromPrimitive)]
pub enum MetastableState {
    #[serde(rename = "m")]
    M = 1,
    #[serde(rename = "n")]
    N,
}

serde_plain::derive_fromstr_from_deserialize!(MetastableState, |e| -> Error {
    Error::InvalidState(e.to_string())
});
serde_plain::derive_display_from_serialize!(MetastableState);

#[derive(Debug, Clone)]
pub struct Progeny {
    pub nuclide: Nuclide,
    pub branch_rate: f64,
    pub decay_mode: DecayModeSet,
}

pub mod decay_mode {
    use std::ops::BitOr;
    use std::str::FromStr;

    use chumsky::prelude::*;
    use flagset::{flags, FlagSet};
    use serde::{de::Visitor, Deserialize};

    use crate::error::Error;
    use crate::primitive::parser::{decaymode, decaymodeflags};

    flags! {
        #[derive(Deserialize)]
        pub enum DecayMode: u8 {
            #[serde(rename = "A")]
            Alpha,
            #[serde(rename = "B-")]
            BetaMinus,
            #[serde(rename = "B+")]
            BetaPlus,
            #[serde(rename = "EC")]
            ElectronCapture,
            #[serde(rename = "IT")]
            IsometricTransition,
            #[serde(rename = "SF")]
            SpontaneousFission,
        }
    }

    impl FromStr for DecayMode {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            decaymode()
                .then_ignore(end())
                .parse(s)
                .into_result()
                .map_err(|e| e.into())
        }
    }

    impl std::fmt::Display for DecayMode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    Self::Alpha => "⍺",
                    Self::BetaMinus => "β-",
                    Self::BetaPlus => "β+",
                    Self::ElectronCapture => "EC",
                    Self::IsometricTransition => "IT",
                    Self::SpontaneousFission => "SF",
                }
            )
        }
    }

    #[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq, Eq)]
    pub struct DecayModeSet(
        #[serde(deserialize_with = "self::deserialize")] pub FlagSet<DecayMode>,
    );

    impl BitOr<DecayMode> for DecayModeSet {
        type Output = Self;

        fn bitor(self, rhs: DecayMode) -> Self::Output {
            Self(self.0 | rhs)
        }
    }

    impl std::fmt::Display for DecayModeSet {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let modes: Vec<String> = self.0.into_iter().map(|m| m.to_string()).collect();
            write!(f, "{}", modes.join("|"))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<FlagSet<DecayMode>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DecayModeVisitor;

        impl<'de> Visitor<'de> for DecayModeVisitor {
            type Value = FlagSet<DecayMode>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A|B-|B+|EC|IT|SF")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                decaymodeflags()
                    .then_ignore(end())
                    .parse(v)
                    .into_result()
                    .map_err(|_| serde::de::Error::custom("Invalid decay mode"))
            }
        }

        deserializer.deserialize_str(DecayModeVisitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum TimeUnit {
    #[serde(rename = "us")]
    MicroSecond,
    #[serde(rename = "ms")]
    MilliSecond,
    #[serde(rename = "s")]
    Second,
    #[serde(rename = "m")]
    Minute,
    #[serde(rename = "h")]
    Hour,
    #[serde(rename = "d")]
    Day,
    #[serde(rename = "y")]
    Year,
}

serde_plain::derive_fromstr_from_deserialize!(TimeUnit);

impl std::fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::MicroSecond => "μs",
                Self::MilliSecond => "ms",
                Self::Second => "s",
                Self::Minute => "m",
                Self::Hour => "h",
                Self::Day => "d",
                Self::Year => "y",
            }
        )
    }
}

impl TimeUnit {
    pub fn as_sec(&self) -> f64 {
        match self {
            Self::MicroSecond => 1e-6,
            Self::MilliSecond => 1e-3,
            Self::Second => 1.,
            Self::Minute => 60.,
            Self::Hour => 3_600.,
            Self::Day => 86_400.,
            Self::Year => 365.2422 * 86_400.,
        }
    }
}

#[derive(Debug, Clone, Copy, DeserializeFromStr)]
pub struct HalfLife {
    pub value: f64,
    pub unit: TimeUnit,
}

impl HalfLife {
    /// Half-life in seconds
    pub fn as_sec(&self) -> f64 {
        self.value * self.unit.as_sec()
    }

    /// Decay constant (s^-1)
    pub fn as_lambda(&self) -> f64 {
        2.0_f64.ln() / self.as_sec()
    }
}

impl FromStr for HalfLife {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        halflife().then_ignore(end()).parse(s).into_result().map_err(|e| e.into())
    }
}

impl std::fmt::Display for HalfLife {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let number_str = PrettyPrintFloat(self.value).to_string();
        let number_str = number_str.strip_suffix(".0").unwrap_or(&number_str);
        write!(f, "{} {}", number_str, self.unit)
    }
}

impl PartialEq for HalfLife {
    fn eq(&self, other: &Self) -> bool {
        self.as_sec() == other.as_sec()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nuclide_from_string() {
        let i131: Nuclide = "I-131".parse().unwrap();
        assert_eq!(i131.id().unwrap(), 531310000);

        let tc99m: Nuclide = "Tc-99m".parse().unwrap();
        assert_eq!(tc99m.id().unwrap(), 430990001);

        let cc99: Result<Nuclide, Error> = "Cc-99".parse();
        assert!(cc99.is_err());
    }

    #[test]
    fn nuclide_to_string() {
        let i131: Nuclide = "I-131".parse().unwrap();
        assert_eq!(&i131.to_string(), "I-131");

        let tc99m: Nuclide = "Tc-99m".parse().unwrap();
        assert_eq!(&tc99m.to_string(), "Tc-99m");
    }

    #[test]
    fn deserialize_decay_mode() {
        let de = serde_plain::Deserializer::new("A ECB-");
        let mode = decay_mode::deserialize(de).unwrap();

        assert_eq!(
            mode,
            DecayMode::Alpha | DecayMode::ElectronCapture | DecayMode::BetaMinus
        );
    }

    fn isclose(a: f64, b: f64) -> bool {
        (a - b).abs() <= f64::EPSILON
    }

    #[test]
    fn halflife_from_string() {
        let t1: HalfLife = "1 us".parse().unwrap();
        assert!(isclose(t1.value, 1.));
        assert_eq!(t1.unit, TimeUnit::MicroSecond);

        let t2: HalfLife = "2h".parse().unwrap();
        assert!(isclose(t2.value, 2.));
        assert_eq!(t2.unit, TimeUnit::Hour);

        let t3: HalfLife = "10y".parse().unwrap();
        assert!(isclose(t3.value, 10.));
        assert_eq!(t3.unit, TimeUnit::Year);

        let t4: HalfLife = "1.1 s".parse().unwrap();
        assert!(isclose(t4.value, 1.1));
        assert_eq!(t4.unit, TimeUnit::Second);
    }

    #[test]
    fn halflife_to_string() {
        let t1: HalfLife = "1us".parse().unwrap();
        assert_eq!(t1.to_string(), "1 μs");

        let t2: HalfLife = "10y".parse().unwrap();
        assert_eq!(t2.to_string(), "10 y");

        let t3: HalfLife = "1.1s".parse().unwrap();
        assert_eq!(t3.to_string(), "1.1 s");
    }

    #[test]
    fn halflife_as_sec() {
        let t1: HalfLife = "1us".parse().unwrap();
        assert!(isclose(t1.as_sec(), 1e-6));

        let t2: HalfLife = "10y".parse().unwrap();
        assert!(isclose(t2.as_sec(), 10. * 365.2422 * 86400.));
    }
}
