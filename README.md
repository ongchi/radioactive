# Radioactive

Simple library for radiation protection data.

This library provides wrappers for various data sources for easy access in Rust:

- [Radiological Toolbox](https://www.ornl.gov/crpk/software)
- [Nuclear Decay Data for Dosimetric Calculations](https://www.icrp.org/publication.asp?id=ICRP%20Publication%20107)
- [X-Ray Mass Attenuation Coefficients](https://www.nist.gov/pml/x-ray-mass-attenuation-coefficients)

## Features

- `radtoolbox` (default): the Radiological Toolbox databases. It needs `mdbsql`,
  a C library, thus it does not compile for WebAssembly. With
  `--no-default-features`, the NIST part compiles for `wasm32-unknown-unknown`.

## X-ray mass attenuation coefficients

The NIST tables of J. H. Hubbell and S. M. Seltzer (NIST Standard Reference
Database 126) for the elements Z = 1 to 92, from 1 keV to 20 MeV, are not in
the repository. `data/download.py --dataset nist_mass_attenuation_coefficient`
(run in `data`) writes them to `data/XrayMassAttenCoef`: Table 1
(`material_constants`) and Table 3 (one file for each element, `01` to `92`).
The tests of this data are ignored by default; run them with
`cargo test -- --ignored` after the download.

- `NistMassAttenCoef::open(path)` reads the files. `from_texts` takes their
  text: for a program without file access (WebAssembly).
- `element_mass_attenuation_coefficient_at` and `mass_attenuation_coefficient_at`
  give μ/ρ (cm²/g) at any energy (eV) of the table range, by log-log
  interpolation. An absorption edge has two rows with the same energy: below the
  edge energy the value comes from the lower row, at and above it from the upper
  row.
- `MassAttenuationCoefficient::mass_attenuation_coefficient` gives the value at a
  table energy only.
