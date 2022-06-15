mod ndx;
mod reader;
mod spectrum;

use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::decay_chain::DecayChain;
use crate::error::Error;
use crate::nuclide::{HalfLife, Nuclide, Progeny};
use reader::{IndexReader, SpectrumReader};
use spectrum::{ack, bet, nsf, rad};

static NDX: OnceCell<HashMap<Nuclide, ndx::Attribute>> = OnceCell::new();
static RAD: OnceCell<HashMap<Nuclide, Vec<rad::RadSpectrum>>> = OnceCell::new();
static BET: OnceCell<HashMap<Nuclide, Vec<bet::BetSpectrum>>> = OnceCell::new();
static ACK: OnceCell<HashMap<Nuclide, Vec<ack::AckSpectrum>>> = OnceCell::new();
static NSF: OnceCell<HashMap<Nuclide, Vec<nsf::NsfSpectrum>>> = OnceCell::new();

pub struct NuclideData {
    path: PathBuf,
}

impl NuclideData {
    pub fn open<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn ndx(&self) -> Result<&HashMap<Nuclide, ndx::Attribute>, Error> {
        NDX.get_or_try_init(|| IndexReader::new(&self.path.join("ICRP-07.NDX")).read())
    }

    pub fn rad(&self) -> Result<&HashMap<Nuclide, Vec<rad::RadSpectrum>>, Error> {
        RAD.get_or_try_init(|| SpectrumReader::new(&self.path.join("ICRP-07.RAD")).read())
    }

    pub fn bet(&self) -> Result<&HashMap<Nuclide, Vec<bet::BetSpectrum>>, Error> {
        BET.get_or_try_init(|| SpectrumReader::new(&self.path.join("ICRP-07.BET")).read())
    }

    pub fn ack(&self) -> Result<&HashMap<Nuclide, Vec<ack::AckSpectrum>>, Error> {
        ACK.get_or_try_init(|| SpectrumReader::new(&self.path.join("ICRP-07.ACK")).read())
    }

    pub fn nsf(&self) -> Result<&HashMap<Nuclide, Vec<nsf::NsfSpectrum>>, Error> {
        NSF.get_or_try_init(|| SpectrumReader::new(&self.path.join("ICRP-07.NSF")).read())
    }
}

impl DecayChain for NuclideData {
    fn get_progeny(&self, nuclide: &Nuclide) -> Option<Vec<Progeny>> {
        self.ndx()
            .unwrap()
            .get(nuclide)
            .map(|attr| attr.progeny.clone())
    }

    fn get_half_life(&self, nuclide: &Nuclide) -> Option<HalfLife> {
        self.ndx().unwrap().get(nuclide).map(|attr| attr.half_life)
    }
}