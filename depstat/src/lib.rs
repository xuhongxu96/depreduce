use std::collections::{HashMap, HashSet};

use depreduce::{
    graph::DependencyGraph,
    reducers::reduce_context::{Operation, ReductionAttempt},
};
use utils::from_json_lines;

#[derive(Debug)]
pub struct Addition {
    pub from: String,
    pub to: String,

    pub reason: String,
}

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct Removal {
    pub from: String,
    pub to: String,
}

#[derive(Debug)]
pub struct AnalysisResult {
    pub removals: HashMap<Removal, Vec<Addition>>,

    pub n_removals: usize,
    pub n_direct_removals: usize,
    pub n_lifting: usize,
    pub n_flattening: usize,

    pub n_targets: usize,
    pub n_edges: usize,
    pub n_considered_edges: usize,
    pub n_builds: usize,
}

impl AnalysisResult {
    fn new(graph: &DependencyGraph, attempts: &Vec<ReductionAttempt>) -> Self {
        struct OpReason {
            attempt: (usize, usize),
            reason: String,
        }

        let mut added: HashSet<(usize, usize)> = HashSet::new();
        let mut removed: HashSet<(usize, usize)> = HashSet::new();
        let mut reasons: HashMap<(usize, usize), OpReason> = HashMap::new();

        for attempt in attempts {
            if attempt.ops.is_empty() {
                continue;
            }

            if !matches!(attempt.ops.last().unwrap(), Operation::Commit(_)) {
                continue;
            }

            let mut build_times = 0;

            for op in &attempt.ops {
                match op {
                    Operation::Build(_) => {
                        build_times += 1;
                    }
                    Operation::Add(add) => {
                        if !removed.remove(&(add.dependent_node_id, add.node_id)) {
                            added.insert((add.dependent_node_id, add.node_id));
                        }
                        reasons.insert(
                            (add.dependent_node_id, add.node_id),
                            OpReason {
                                attempt: (attempt.candidates.dependent, attempt.candidates.node_id),
                                reason: format!(
                                    "{}",
                                    match build_times {
                                        1 | 2 => "deplift", // build times may be 1 when timeout
                                        3 => "depflatten",
                                        _ => {
                                            eprintln!("{:#?}", attempt);
                                            eprintln!("build times: {}", build_times);
                                            unreachable!()
                                        }
                                    }
                                ),
                            },
                        );
                    }
                    Operation::Remove(remove) => {
                        if !added.remove(&(remove.dependent_node_id, remove.node_id)) {
                            removed.insert((remove.dependent_node_id, remove.node_id));
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut removals: HashMap<Removal, Vec<Addition>> = HashMap::new();
        for (dependent, node_id) in removed.iter() {
            removals.insert(
                Removal {
                    from: graph.nodes[*dependent].label.clone(),
                    to: graph.nodes[*node_id].label.clone(),
                },
                vec![],
            );
        }

        for (dependent, node_id) in added.iter() {
            let reason = reasons.get(&(*dependent, *node_id)).unwrap();
            let mut reason_removal = reason.attempt;
            while !removed.contains(&reason_removal) {
                reason_removal = match reasons.get(&reason_removal) {
                    Some(r) => r.attempt,
                    None => break,
                };
            }

            assert!(removed.contains(&reason_removal));
            removals
                .get_mut(&Removal {
                    from: graph.nodes[reason_removal.0].label.clone(),
                    to: graph.nodes[reason_removal.1].label.clone(),
                })
                .unwrap()
                .push(Addition {
                    from: graph.nodes[*dependent].label.clone(),
                    to: graph.nodes[*node_id].label.clone(),
                    reason: reason.reason.clone(),
                });
        }

        let n_removals = removals.len();
        let n_direct_removals = removals.iter().filter(|(_, adds)| adds.is_empty()).count();
        let n_lifting = removals
            .iter()
            .map(|(_, adds)| adds.iter().filter(|a| a.reason == "deplift").count())
            .sum();
        let n_flattening = removals
            .iter()
            .map(|(_, adds)| adds.iter().filter(|a| a.reason == "depflatten").count())
            .sum();

        AnalysisResult {
            removals,
            n_removals,
            n_direct_removals,
            n_lifting,
            n_flattening,
            n_targets: attempts
                .iter()
                .map(|a| a.candidates.node_id)
                .collect::<HashSet<_>>()
                .len(),
            n_edges: attempts.len(),
            n_considered_edges: attempts.iter().filter(|a| !a.ops.is_empty()).count(),
            n_builds: attempts
                .iter()
                .map(|a| {
                    a.ops
                        .iter()
                        .filter(|op| matches!(op, Operation::Build(_)))
                        .count()
                })
                .sum(),
        }
    }
}

pub fn parse_logs(log_dir: &str) -> AnalysisResult {
    let graph_path = format!("{}/00-graph.json", log_dir);
    let attempts_path = format!("{}/01-attempts.jsonl", log_dir);

    let graph: DependencyGraph =
        serde_json::from_str(&std::fs::read_to_string(&graph_path).unwrap()).unwrap();

    let attempts: Vec<ReductionAttempt> =
        from_json_lines(&std::fs::read_to_string(&attempts_path).unwrap()).collect();

    AnalysisResult::new(&graph, &attempts)
}
