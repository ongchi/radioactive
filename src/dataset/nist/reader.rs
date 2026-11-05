use std::collections::BTreeMap;

use super::{MassAttenCoef, MassAttenCoefRecord, MaterialConstant, MaterialConstantRecord};

use crate::error::Error;
use crate::primitive::Symbol;

/// Rows of a table after its two header lines
fn table_rows(text: &str) -> impl Iterator<Item = &str> {
    text.lines().skip(2).filter(|line| !line.trim().is_empty())
}

pub fn read_material_constants(text: &str) -> Result<BTreeMap<Symbol, MaterialConstant>, Error> {
    let mut content = BTreeMap::new();

    for line in table_rows(text) {
        let row: MaterialConstantRecord =
            fixed_width::from_str(line).map_err(|e| Error::Unexpected(e.into()))?;
        content.insert(row.symbol, row.into());
    }

    Ok(content)
}

/// Rows in the order of the table (increasing energy); an absorption edge has two
/// rows with the same energy, below and above the edge
pub fn read_mass_atten_coef(text: &str) -> Result<Vec<MassAttenCoef>, Error> {
    table_rows(text)
        .map(|line| {
            fixed_width::from_str::<MassAttenCoefRecord>(line)
                .map(Into::into)
                .map_err(|e| Error::Unexpected(e.into()))
        })
        .collect()
}
