//! EXPLAIN plan analysis and visualization.

use tg_core::error::CoreResult;
use tg_core::traits::driver::Connection;
use tg_core::types::query::{ExecutionPlan, ExecutionPlanNode, ExplainFormat};

/// Analyze a query's execution plan and return optimization suggestions.
#[must_use]
pub fn analyze_plan(plan: &ExecutionPlan) -> PlanAnalysis {
    let mut warnings = Vec::new();
    let mut suggestions = Vec::new();
    let mut total_cost = 0.0;

    analyze_node(&plan.root, &mut warnings, &mut suggestions, &mut total_cost);

    PlanAnalysis {
        total_cost_estimate: total_cost,
        warnings,
        suggestions,
        node_count: count_nodes(&plan.root),
    }
}

/// The result of plan analysis.
#[derive(Clone, Debug)]
pub struct PlanAnalysis {
    /// Total estimated cost.
    pub total_cost_estimate: f64,
    /// Performance warnings.
    pub warnings: Vec<String>,
    /// Optimization suggestions.
    pub suggestions: Vec<String>,
    /// Total number of nodes in the plan tree.
    pub node_count: usize,
}

fn analyze_node(
    node: &ExecutionPlanNode,
    warnings: &mut Vec<String>,
    suggestions: &mut Vec<String>,
    total_cost: &mut f64,
) {
    *total_cost += node.total_cost.unwrap_or(0.0);

    // Check for expensive operations
    let op = node.operation.to_lowercase();

    if op.contains("seq scan") || op.contains("table scan") {
        if node.plan_rows.unwrap_or(0) > 1000 {
            warnings.push(format!(
                "Sequential scan on large table (est. {} rows) — consider adding an index",
                node.plan_rows.unwrap_or(0)
            ));
            suggestions.push("Add an appropriate index for the WHERE clause columns".into());
        }
    }

    if op.contains("nested loop") {
        warnings.push(
            "Nested loop join detected — may be slow for large datasets".into(),
        );
        suggestions
            .push("Consider adding indexes on join columns or rewriting as hash join".into());
    }

    if op.contains("sort") && node.actual_time_ms.unwrap_or(0.0) > 100.0 {
        warnings.push(format!(
            "Sort operation taking {:.1}ms — consider adding indexes to avoid sorting",
            node.actual_time_ms.unwrap_or(0.0)
        ));
    }

    if let Some(cost) = node.total_cost {
        if cost > 10_000.0 {
            warnings.push(format!(
                "High-cost node: {} (cost: {:.1})",
                node.operation, cost
            ));
        }
    }

    // Include original warnings and suggestions from the plan
    warnings.extend(node.warnings.clone());
    suggestions.extend(node.suggestions.clone());

    for child in &node.children {
        analyze_node(child, warnings, suggestions, total_cost);
    }
}

fn count_nodes(node: &ExecutionPlanNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

/// Convert a plan to a Graphviz DOT representation.
#[must_use]
pub fn to_dot(plan: &ExecutionPlan) -> String {
    let mut dot = String::from("digraph execution_plan {\n");
    dot.push_str("  rankdir=TB;\n");
    dot.push_str("  node [shape=box, style=rounded];\n");

    let mut id = 0;
    write_dot_node(&plan.root, &mut dot, &mut id);

    dot.push_str("}\n");
    dot
}

fn write_dot_node(node: &ExecutionPlanNode, dot: &mut String, id: &mut usize) {
    let current = *id;
    *id += 1;

    let label = if let Some(ref desc) = node.description {
        format!("{}\\n{}", node.operation, desc.chars().take(60).collect::<String>())
    } else {
        node.operation.clone()
    };

    let rows = node.plan_rows.map_or("?".into(), |r| r.to_string());
    let cost = node.total_cost.map_or("?".into(), |c| format!("{c:.1}"));

    dot.push_str(&format!(
        "  n{current} [label=\"{label}\\nrows={rows} cost={cost}\"];\n"
    ));

    for child in &node.children {
        let child_id = *id;
        write_dot_node(child, dot, id);
        dot.push_str(&format!("  n{current} -> n{child_id};\n"));
    }
}

/// Create a simple text-tree representation of the plan.
#[must_use]
pub fn to_tree(plan: &ExecutionPlan) -> String {
    let mut out = String::new();
    write_tree_node(&plan.root, &mut out, "", true);
    out
}

fn write_tree_node(
    node: &ExecutionPlanNode,
    out: &mut String,
    prefix: &str,
    _is_last: bool,
) {
    let rows = node.plan_rows.map_or("?".into(), |r| r.to_string());
    let cost = node.total_cost.map_or("?".into(), |c| format!("{c:.1}"));

    let line = format!(
        "{prefix}{} (rows={rows}, cost={cost})",
        node.operation
    );
    out.push_str(&line);
    out.push('\n');

    for (i, child) in node.children.iter().enumerate() {
        let is_last = i == node.children.len() - 1;
        let new_prefix = if is_last {
            format!("{prefix}  └─ ")
        } else {
            format!("{prefix}  ├─ ")
        };
        write_tree_node(child, out, &new_prefix, is_last);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tg_core::types::query::ExecutionPlanNode;
    use std::collections::HashMap;

    fn sample_plan() -> ExecutionPlan {
        ExecutionPlan {
            root: ExecutionPlanNode {
                operation: "Hash Join".into(),
                description: None,
                startup_cost: Some(0.0),
                total_cost: Some(150.0),
                plan_rows: Some(500),
                plan_width: Some(64),
                actual_time_ms: Some(12.5),
                actual_rows: Some(500),
                memory_kb: Some(1024),
                properties: HashMap::new(),
                children: vec![
                    ExecutionPlanNode {
                        operation: "Seq Scan on users".into(),
                        description: Some("Filter: (age > 18)".into()),
                        startup_cost: Some(0.0),
                        total_cost: Some(5000.0),
                        plan_rows: Some(100000),
                        plan_width: Some(32),
                        actual_time_ms: Some(200.0),
                        actual_rows: Some(100000),
                        memory_kb: None,
                        properties: HashMap::new(),
                        children: Vec::new(),
                        warnings: Vec::new(),
                        suggestions: Vec::new(),
                    },
                ],
                warnings: Vec::new(),
                suggestions: Vec::new(),
            },
            planning_time_ms: Some(1.2),
            execution_time_ms: Some(212.5),
            triggers: None,
            raw_text: None,
            format: ExplainFormat::Text,
        }
    }

    #[test]
    fn test_analyze_plan() {
        let analysis = analyze_plan(&sample_plan());
        assert!(!analysis.warnings.is_empty());
        assert!(analysis.warnings.iter().any(|w| w.contains("Sequential scan")));
    }

    #[test]
    fn test_to_tree() {
        let tree = to_tree(&sample_plan());
        assert!(tree.contains("Hash Join"));
        assert!(tree.contains("Seq Scan"));
    }

    #[test]
    fn test_to_dot() {
        let dot = to_dot(&sample_plan());
        assert!(dot.starts_with("digraph"));
        assert!(dot.contains("Hash Join"));
    }
}
