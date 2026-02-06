use chumsky::prelude::*;
use flagset::FlagSet;

use super::notation::{Compound, Symbol};
use super::nuclide::{DecayMode, HalfLife, MetastableState, Nuclide, TimeUnit};

pub fn symbol<'src>() -> impl Parser<'src, &'src str, Symbol, extra::Err<Rich<'src, char>>> + Clone
{
    any()
        .filter(|c: &char| c.is_ascii_uppercase())
        .then(
            any()
                .filter(|c: &char| c.is_ascii_lowercase())
                .repeated()
                .collect::<Vec<_>>(),
        )
        .try_map(|(first, rest): (char, Vec<char>), span| {
            let mut s = String::with_capacity(1 + rest.len());
            s.push(first);
            s.extend(rest);
            s.parse::<Symbol>()
                .map_err(|e| Rich::custom(span, format!("{}", e)))
        })
}

pub fn nuclide<'src>() -> impl Parser<'src, &'src str, Nuclide, extra::Err<Rich<'src, char>>> {
    let sf = just("SF").map(|_| Nuclide::FissionProducts);

    let symbol = symbol().map(|s| s as u32 * 10_000_000);

    let mass = text::int(10)
        .try_map(|s: &str, span| {
            s.parse::<u32>()
                .map_err(|e| Rich::custom(span, format!("{}", e)))
        })
        .map(|m| m * 10_000);

    let meta = any()
        .filter(|c: &char| c.is_ascii_alphabetic())
        .or_not()
        .try_map(|meta, span| {
            if let Some(c) = meta {
                String::from(c)
                    .parse::<MetastableState>()
                    .map_err(|e| Rich::custom(span, format!("{}", e)))
                    .map(|m| m as u32)
            } else {
                Ok(0)
            }
        });

    let with_name = sf.or(symbol
        .then(just('-').or_not().map(|_| 0u32))
        .then(mass)
        .then(meta)
        .map(|(((sym, _dash), mass), meta)| Nuclide::WithId(sym + mass + meta)));

    let with_id = text::int(10).try_map(|s: &str, span| {
        s.parse::<u32>()
            .map_err(|e| Rich::custom(span, format!("{}", e)))
            .map(Nuclide::WithId)
    });

    with_name.or(with_id)
}

pub fn decaymode<'src>(
) -> impl Parser<'src, &'src str, DecayMode, extra::Err<Rich<'src, char>>> {
    let a = just("A").or(just("⍺")).map(|_| DecayMode::Alpha).padded();
    let bm = just("B-")
        .or(just("β-"))
        .map(|_| DecayMode::BetaMinus)
        .padded();
    let bp = just("B+")
        .or(just("β+"))
        .map(|_| DecayMode::BetaPlus)
        .padded();
    let ec = just("EC").map(|_| DecayMode::ElectronCapture).padded();
    let it = just("IT").map(|_| DecayMode::IsometricTransition).padded();
    let sf = just("SF").map(|_| DecayMode::SpontaneousFission).padded();

    a.or(bm.or(bp.or(ec.or(it.or(sf)))))
}

pub fn decaymodeflags<'src>(
) -> impl Parser<'src, &'src str, FlagSet<DecayMode>, extra::Err<Rich<'src, char>>> {
    decaymode()
        .repeated()
        .collect::<Vec<_>>()
        .map(|modes| modes.into_iter().fold(FlagSet::default(), |a, b| a | b))
}

pub fn compound<'src>(
) -> impl Parser<'src, &'src str, Compound, extra::Err<Rich<'src, char>>> {
    let number = any()
        .filter(|c: &char| c.is_ascii_digit())
        .repeated()
        .collect::<Vec<_>>()
        .map(|s| {
            s.into_iter()
                .collect::<String>()
                .parse()
                .unwrap_or(1)
        });

    let compound = recursive(|expr| {
        symbol()
            .then(number)
            .map(|(s, n)| Compound::Element(s, n))
            .or(expr
                .delimited_by(just('('), just(')'))
                .then(number)
                .map(|(mole, n)| Compound::Molecule(mole, n)))
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
    });

    compound.map(|mole| {
        if mole.len() == 1 {
            mole.into_iter().next().unwrap()
        } else {
            Compound::Molecule(mole, 1)
        }
    })
}

pub fn float<'src>() -> impl Parser<'src, &'src str, f64, extra::Err<Rich<'src, char>>> {
    let sign = one_of("+-").or_not();
    let digits = any().filter(|c: &char| c.is_ascii_digit()).repeated();

    // sign? digits+ ('.' digits*)?  ('e'|'E' sign? digits+)?
    sign.then(digits.at_least(1))
        .then(just('.').then(digits).or_not())
        .then(
            one_of("eE")
                .then(one_of("+-").or_not())
                .then(digits.at_least(1))
                .or_not(),
        )
        .to_slice()
        .try_map(|s: &str, span| {
            s.parse::<f64>()
                .map_err(|e| Rich::custom(span, format!("{}", e)))
        })
}

pub fn halflife<'src>(
) -> impl Parser<'src, &'src str, HalfLife, extra::Err<Rich<'src, char>>> {
    let us = just("us").map(|_| TimeUnit::MicroSecond);
    let ms = just("ms").map(|_| TimeUnit::MilliSecond);
    let s = just("s").map(|_| TimeUnit::Second);
    let m = just("m").map(|_| TimeUnit::Minute);
    let h = just("h").map(|_| TimeUnit::Hour);
    let d = just("d").map(|_| TimeUnit::Day);
    let y = just("y").map(|_| TimeUnit::Year);

    let unit = us.or(ms.or(s.or(m.or(h.or(d.or(y))))));

    float()
        .padded()
        .then(unit)
        .map(|(value, unit)| HalfLife { value, unit })
}

pub fn gi_absorption_factor<'src>(
) -> impl Parser<'src, &'src str, (f64, String), extra::Err<Rich<'src, char>>> {
    let compound = any()
        .filter(|c: &char| c.is_ascii_alphanumeric())
        .repeated()
        .collect::<Vec<_>>()
        .map(|s| s.into_iter().collect::<String>());

    float().padded().then(compound)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_symbol() {
        let h = symbol().parse("H").into_result().unwrap();
        assert_eq!(h, Symbol::H);

        let he = symbol().parse("He").into_result().unwrap();
        assert_eq!(he, Symbol::He);

        let cc = symbol().parse("Cc").into_result();
        assert!(cc.is_err());
    }

    #[test]
    fn parse_nuclide() {
        let tc99 = nuclide().parse("Tc99").into_result().unwrap();
        assert_eq!(tc99, Nuclide::WithId(43_099_0000));

        let tc99m = nuclide().parse("Tc-99m").into_result().unwrap();
        assert_eq!(tc99m, Nuclide::WithId(43_099_0001));

        let tc99m_from_id = nuclide().parse("430990001").into_result().unwrap();
        assert_eq!(tc99m_from_id, Nuclide::WithId(43_099_0001));

        let tc99o = nuclide().parse("Tc-99o").into_result();
        assert!(tc99o.is_err());

        let cc99 = nuclide().parse("Cc-99").into_result();
        assert!(cc99.is_err());
    }

    #[test]
    fn parse_decaymodeflags() {
        // formatter.write_str("A|B-|B+|EC|IT|SF")
        let mode = decaymodeflags()
            .parse("AB-B+ECITSF")
            .into_result()
            .unwrap();
        assert_eq!(
            mode,
            FlagSet::default()
                | DecayMode::Alpha
                | DecayMode::BetaMinus
                | DecayMode::BetaPlus
                | DecayMode::ElectronCapture
                | DecayMode::IsometricTransition
                | DecayMode::SpontaneousFission
        );

        let mode_with_padding = decaymodeflags()
            .parse("A B- β+ ")
            .into_result()
            .unwrap();
        assert_eq!(
            mode_with_padding,
            FlagSet::default() | DecayMode::Alpha | DecayMode::BetaMinus | DecayMode::BetaPlus
        );
    }

    #[test]
    fn parse_compound() {
        let ether = compound().parse("(C2H5)2O").into_result().unwrap();
        assert_eq!(
            ether,
            Compound::Molecule(
                vec![
                    Compound::Molecule(
                        vec![
                            Compound::Element(Symbol::C, 2),
                            Compound::Element(Symbol::H, 5),
                        ],
                        2
                    ),
                    Compound::Element(Symbol::O, 1)
                ],
                1
            )
        );
    }

    #[test]
    fn parse_float() {
        let f1 = float().parse("1").into_result().unwrap();
        assert_eq!(f1, 1.0);

        let f2 = float().parse("2.").into_result().unwrap();
        assert_eq!(f2, 2.0);

        let f3 = float().parse("1.2").into_result().unwrap();
        assert_eq!(f3, 1.2);

        let f4 = float().parse("1.23e01").into_result().unwrap();
        assert_eq!(f4, 12.3);

        let f5 = float().parse("-1.234e-2").into_result().unwrap();
        assert_eq!(f5, -0.01234);
    }

    #[test]
    fn parse_halflife() {
        let h1 = halflife().parse("1.23e-2s").into_result().unwrap();
        assert_eq!(
            h1,
            HalfLife {
                value: 0.0123,
                unit: TimeUnit::Second
            }
        );

        let h2 = halflife().parse("321 h").into_result().unwrap();
        assert_eq!(
            h2,
            HalfLife {
                value: 321.,
                unit: TimeUnit::Hour
            }
        )
    }

    #[test]
    fn parse_gi_absorption_factor() {
        let f1_1 = gi_absorption_factor()
            .parse("1OBT")
            .into_result()
            .unwrap();
        assert_eq!(f1_1, (1., "OBT".to_string()));

        let f1_2 = gi_absorption_factor()
            .parse("1 CH4")
            .into_result()
            .unwrap();
        assert_eq!(f1_2, (1., "CH4".to_string()));
    }
}
