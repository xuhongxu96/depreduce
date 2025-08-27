use std::collections::{HashMap, HashSet};

use depreduce::{
    graph::DependencyGraph,
    reducers::reduce_context::{Operation, ReductionAttempt},
};
use utils::from_json_lines;

pub fn parse_logs(log_dir: &str) {
    struct OpReason {
        attempt: (usize, usize),
        reason: String,
    }

    let graph_path = format!("{}/00-graph.json", log_dir);
    let attempts_path = format!("{}/01-attempts.jsonl", log_dir);

    let graph: DependencyGraph =
        serde_json::from_str(&std::fs::read_to_string(&graph_path).unwrap()).unwrap();

    let attempts: Vec<ReductionAttempt> =
        from_json_lines(&std::fs::read_to_string(&attempts_path).unwrap()).collect();

    println!("Parsed {} reduction attempts from logs.", attempts.len());

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
                                    2 => "deplift",
                                    3 => "depflatten",
                                    _ => unreachable!(),
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

    println!("Added dependencies ({}):", added.len());
    for (dependent, node_id) in added.iter() {
        println!(
            "{} -> {}, reason: ",
            graph.nodes[*dependent].label, graph.nodes[*node_id].label
        );

        let reason = reasons.get(&(*dependent, *node_id)).unwrap();
        let mut reason_removal = reason.attempt;
        while !removed.contains(&reason_removal) {
            reason_removal = match reasons.get(&reason_removal) {
                Some(r) => r.attempt,
                None => break,
            };
        }

        println!(
            "  {} when {} -> {} ({})",
            reason.reason,
            graph.nodes[reason_removal.0].label,
            graph.nodes[reason_removal.1].label,
            removed.contains(&reason_removal)
        );
    }

    println!("\nRemoved dependencies ({}):", removed.len());
    for (dependent, node_id) in removed.iter() {
        println!(
            "{} -> {}",
            graph.nodes[*dependent].label, graph.nodes[*node_id].label
        )
    }
}
