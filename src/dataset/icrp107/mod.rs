mod ndx;
mod reader;
pub mod spectrum;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use once_cell::sync::OnceCell;

use crate::error::Error;
use crate::primitive::attr::{NuclideDecayMode, NuclideHalfLife, NuclideProgeny};
use crate::primitive::{DecayModeSet, HalfLife, Nuclide, Progeny};
use reader::{IndexReader, SpectrumReader};
use spectrum::{ack, bet, nsf, rad};

#[derive(Debug)]
pub struct Icrp107 {
    path: PathBuf,
    ndx: OnceCell<HashMap<Nuclide, ndx::Attribute>>,
    rad: OnceCell<HashMap<Nuclide, Vec<rad::RadSpectrum>>>,
    bet: OnceCell<HashMap<Nuclide, Vec<bet::BetSpectrum>>>,
    ack: OnceCell<HashMap<Nuclide, Vec<ack::AckSpectrum>>>,
    nsf: OnceCell<HashMap<Nuclide, Vec<nsf::NsfSpectrum>>>,
}

impl Icrp107 {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path_buf = path.as_ref().to_path_buf();

        if path_buf.is_dir() {
            Ok(Self {
                path: path_buf,
                ndx: OnceCell::new(),
                rad: OnceCell::new(),
                bet: OnceCell::new(),
                ack: OnceCell::new(),
                nsf: OnceCell::new(),
            })
        } else {
            Err(Error::InvalidFilePath)
        }
    }

    pub fn ndx(&self) -> Result<&HashMap<Nuclide, ndx::Attribute>, Error> {
        self.ndx
            .get_or_try_init(|| IndexReader::new(&self.path.join("ICRP-07.NDX"))?.read())
    }

    pub fn rad(&self) -> Result<&HashMap<Nuclide, Vec<rad::RadSpectrum>>, Error> {
        self.rad
            .get_or_try_init(|| SpectrumReader::new(&self.path.join("ICRP-07.RAD"))?.read())
    }

    pub fn bet(&self) -> Result<&HashMap<Nuclide, Vec<bet::BetSpectrum>>, Error> {
        self.bet
            .get_or_try_init(|| SpectrumReader::new(&self.path.join("ICRP-07.BET"))?.read())
    }

    pub fn ack(&self) -> Result<&HashMap<Nuclide, Vec<ack::AckSpectrum>>, Error> {
        self.ack
            .get_or_try_init(|| SpectrumReader::new(&self.path.join("ICRP-07.ACK"))?.read())
    }

    pub fn nsf(&self) -> Result<&HashMap<Nuclide, Vec<nsf::NsfSpectrum>>, Error> {
        self.nsf
            .get_or_try_init(|| SpectrumReader::new(&self.path.join("ICRP-07.NSF"))?.read())
    }
}

impl NuclideProgeny for Icrp107 {
    fn progeny(&self, nuclide: Nuclide) -> Result<Vec<Progeny>, Error> {
        self.ndx()?
            .get(&nuclide)
            .map(|attr| attr.progeny.clone())
            .ok_or_else(|| Error::InvalidNuclide(nuclide.to_string()))
    }
}

impl NuclideHalfLife for Icrp107 {
    fn half_life(&self, nuclide: Nuclide) -> Result<HalfLife, Error> {
        self.ndx()?
            .get(&nuclide)
            .map(|attr| attr.half_life)
            .ok_or_else(|| Error::InvalidNuclide(nuclide.to_string()))
    }
}

impl NuclideDecayMode for Icrp107 {
    fn decay_mode(&self, nuclide: Nuclide) -> Result<DecayModeSet, Error> {
        self.ndx()?
            .get(&nuclide)
            .map(|attr| attr.decay_mode)
            .ok_or_else(|| Error::InvalidNuclide(nuclide.to_string()))
    }
}
