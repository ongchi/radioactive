use crate::primitive::attr::Energy;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid atomic number: {0}")]
    InvalidAtomicNumber(u8),
    #[error("invalid symbol: {0}")]
    InvalidSymbol(String),
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("invalid nuclide: {0}")]
    InvalidNuclide(String),
    #[error("invalid half life: {0}")]
    InvalidHalfLife(String),
    #[error("invalid decay mode: {0}")]
    InvalidDecayMode(String),
    #[error("invalid radiation type: {0}")]
    InvalidRadiationType(String),
    #[error("invalid float number: {0}")]
    InvalidFloat(String),
    #[error("invalid integer: {0}")]
    InvalidInteger(String),
    #[error("invalid energy: {0}")]
    InvalidEnergy(Energy),
    #[error("invalid age group: {0}")]
    InvalidAgeGroup(String),
    #[error("invalid pathway: {0}")]
    InvalidPathway(String),
    #[error("invalid organ: {0}")]
    InvalidOrgan(String),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error("invalid file path")]
    InvalidFilePath,
    #[error("invalid mdb file")]
    InvalidMdbFile,
    #[error(transparent)]
    MdbSqlError(#[from] mdbsql::Error),
}

impl From<Vec<chumsky::prelude::Rich<'_, char>>> for Error {
    fn from(e: Vec<chumsky::prelude::Rich<'_, char>>) -> Self {
        let msg = e
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Error::Unexpected(anyhow::anyhow!("parse error: {}", msg))
    }
}
