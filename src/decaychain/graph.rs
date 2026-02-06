use std::collections::HashSet;
use std::sync::Arc;

use float_pretty_print::PrettyPrintFloat;
use petgraph::{graph::NodeIndex, Graph};

use crate::primitive::attr::{NuclideHalfLife, NuclideProgeny};
use crate::primitive::{DecayModeSet, HalfLife, Nuclide};

#[derive(Clone, Copy)]
pub struct ChainNode {
    nuclide: Nuclide,
    half_life: Option<HalfLife>,
}

impl std::fmt::Display for ChainNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.nuclide)?;
        if let Some(t) = self.half_life {
            write!(f, "\n{}", t)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct ChainEdge {
    branch_rate: f64,
    decay_mode: DecayModeSet,
}

impl std::fmt::Display for ChainEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", PrettyPrintFloat(self.branch_rate))?;
        let modes: Vec<String> = self.decay_mode.0.into_iter().map(|m| m.to_string()).collect();
        write!(f, "{}", modes.join(" or "))
    }
}

pub type DecayChain = Graph<ChainNode, ChainEdge>;

pub struct DecayChainBuilder<D> {
    data: Arc<D>,
}

impl<D> DecayChainBuilder<D>
where
    D: NuclideHalfLife + NuclideProgeny,
{
    pub fn new(data: Arc<D>) -> Self {
        Self { data }
    }

    pub fn build(self, root: Nuclide) -> DecayChain {
        let mut graph: Graph<ChainNode, ChainEdge> = Graph::new();

        let mut get_or_insert_node = |nuclide: Nuclide| -> NodeIndex {
            match nuclide {
                Nuclide::WithId(_) => {
                    match graph
                        .raw_nodes()
                        .iter()
                        .position(|n| n.weight.nuclide == nuclide)
                    {
                        Some(i) => NodeIndex::new(i),
                        None => {
                            let half_life = self.data.half_life(nuclide).ok();
                            graph.add_node(ChainNode { nuclide, half_life })
                        }
                    }
                }
                Nuclide::FissionProducts => {
                    let half_life = None;
                    graph.add_node(ChainNode { nuclide, half_life })
                }
            }
        };

        let mut stack: Vec<Nuclide> = vec![root];
        let mut visited = HashSet::new();
        let mut edges = vec![];

        while let Some(parent) = stack.pop() {
            match parent {
                Nuclide::WithId(_) => {
                    visited.insert(parent);

                    if let Ok(progeny) = self.data.progeny(parent) {
                        let p_node = get_or_insert_node(parent);
                        for daughter in progeny {
                            if !visited.contains(&daughter.nuclide) {
                                stack.push(daughter.nuclide)
                            }

                            let d_node = get_or_insert_node(daughter.nuclide);
                            let weight = ChainEdge {
                                branch_rate: daughter.branch_rate,
                                decay_mode: daughter.decay_mode,
                            };
                            edges.push((p_node, d_node, weight));
                        }
                    }
                }
                Nuclide::FissionProducts => {}
            }
        }
        for (p_node, d_node, weight) in edges {
            graph.add_edge(p_node, d_node, weight);
        }

        graph
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::Error;
    use crate::primitive::{DecayMode, DecayModeSet, Progeny, TimeUnit};

    struct TestData {
        pub mo99: Nuclide,
        pub tc99m: Nuclide,
        pub progeny: Vec<Progeny>,
    }

    impl TestData {
        pub fn new() -> Self {
            let mo99 = "Mo-99".parse().unwrap();
            let tc99m = "Tc-99m".parse().unwrap();

            let progeny = Progeny {
                nuclide: tc99m,
                branch_rate: 1.0,
                decay_mode: DecayModeSet::default() | "IT".parse::<DecayMode>().unwrap(),
            };

            Self {
                mo99,
                tc99m,
                progeny: vec![progeny],
            }
        }
    }

    impl NuclideHalfLife for TestData {
        fn half_life(&self, nuclide: Nuclide) -> Result<HalfLife, Error> {
            if nuclide == self.mo99 {
                Ok(HalfLife {
                    value: 2.7489,
                    unit: TimeUnit::Day,
                })
            } else if nuclide == self.tc99m {
                Ok(HalfLife {
                    value: 6.0067,
                    unit: TimeUnit::Hour,
                })
            } else {
                Err(Error::InvalidNuclide("not found".to_string()))
            }
        }
    }

    impl NuclideProgeny for TestData {
        fn progeny(&self, nuclide: Nuclide) -> Result<Vec<Progeny>, Error> {
            if nuclide == self.mo99 {
                Ok(self.progeny.clone())
            } else {
                Err(Error::InvalidNuclide("not found".to_string()))
            }
        }
    }

    #[test]
    fn chain_builder() {
        let data = Arc::new(TestData::new());
        let chain = DecayChainBuilder::new(data.clone()).build(data.mo99);

        let nodes = chain.raw_nodes();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].weight.nuclide, data.mo99);
        assert_eq!(
            nodes[0].weight.half_life,
            Some(HalfLife {
                value: 2.7489,
                unit: TimeUnit::Day
            })
        );
        assert_eq!(nodes[1].weight.nuclide, data.tc99m);
        assert_eq!(
            nodes[1].weight.half_life,
            Some(HalfLife {
                value: 6.0067,
                unit: TimeUnit::Hour,
            })
        );

        let edges = chain.raw_edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].weight.branch_rate, 1.0);
        assert_eq!(
            edges[0].weight.decay_mode,
            DecayModeSet::default() | DecayMode::IsometricTransition
        );
    }
}
