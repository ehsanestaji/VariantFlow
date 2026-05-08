use crate::engine::index::schema::IndexChunk;
use crate::expr::{Comparison, ExprNode, Expression, Field, Literal, Operator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipDecision {
    CanSkip,
    MustScan,
    UnsupportedForIndex,
}

pub(crate) fn plan_chunk(expr: &Expression, chunk: &IndexChunk) -> SkipDecision {
    plan_node(expr.root_node(), chunk)
}

fn plan_node(node: &ExprNode, chunk: &IndexChunk) -> SkipDecision {
    match node {
        ExprNode::Comparison(comparison) => plan_comparison(comparison, chunk),
        ExprNode::SampleAggregate { .. } | ExprNode::CountAggregate { .. } => {
            SkipDecision::UnsupportedForIndex
        }
        ExprNode::And(left, right) => plan_and(plan_node(left, chunk), plan_node(right, chunk)),
        ExprNode::Or(left, right) => plan_or(plan_node(left, chunk), plan_node(right, chunk)),
    }
}

fn plan_and(left: SkipDecision, right: SkipDecision) -> SkipDecision {
    if left == SkipDecision::CanSkip || right == SkipDecision::CanSkip {
        SkipDecision::CanSkip
    } else if left == SkipDecision::UnsupportedForIndex
        || right == SkipDecision::UnsupportedForIndex
    {
        SkipDecision::UnsupportedForIndex
    } else {
        SkipDecision::MustScan
    }
}

fn plan_or(left: SkipDecision, right: SkipDecision) -> SkipDecision {
    if left == SkipDecision::CanSkip && right == SkipDecision::CanSkip {
        SkipDecision::CanSkip
    } else if left == SkipDecision::UnsupportedForIndex
        || right == SkipDecision::UnsupportedForIndex
    {
        SkipDecision::UnsupportedForIndex
    } else {
        SkipDecision::MustScan
    }
}

fn plan_comparison(comparison: &Comparison, chunk: &IndexChunk) -> SkipDecision {
    match (comparison.field(), comparison.literal()) {
        (Field::Qual, Literal::Number(value)) => {
            numeric_decision(chunk.qual_min, chunk.qual_max, comparison.op(), *value)
        }
        (Field::Filter, Literal::String(_)) => filter_decision(comparison.op()),
        (Field::Info { key, index: None }, Literal::Number(value)) if key == b"DP" => {
            numeric_decision(
                chunk.info_dp_min.map(|value| value as f64),
                chunk.info_dp_max.map(|value| value as f64),
                comparison.op(),
                *value,
            )
        }
        (Field::Info { key, index: None }, Literal::Number(value)) if key == b"AF" => {
            if !chunk.info_af_complete {
                SkipDecision::MustScan
            } else {
                numeric_decision(
                    chunk.info_af_min,
                    chunk.info_af_max,
                    comparison.op(),
                    *value,
                )
            }
        }
        (Field::Info { .. }, _) => SkipDecision::UnsupportedForIndex,
        (Field::Format { .. }, _) => SkipDecision::UnsupportedForIndex,
        (Field::Chrom | Field::Pos | Field::Qual | Field::Filter, _) => {
            SkipDecision::UnsupportedForIndex
        }
    }
}

fn numeric_decision(min: Option<f64>, max: Option<f64>, op: Operator, value: f64) -> SkipDecision {
    let (Some(min), Some(max)) = (min, max) else {
        return SkipDecision::MustScan;
    };

    if numeric_comparison_cannot_match(min, max, op, value) {
        SkipDecision::CanSkip
    } else {
        SkipDecision::MustScan
    }
}

fn numeric_comparison_cannot_match(min: f64, max: f64, op: Operator, value: f64) -> bool {
    match op {
        Operator::Gt => max <= value,
        Operator::Gte => max < value,
        Operator::Lt => min >= value,
        Operator::Lte => min > value,
        Operator::Eq => value < min || value > max,
        Operator::Ne => min == value && max == value,
    }
}

fn filter_decision(op: Operator) -> SkipDecision {
    match op {
        Operator::Eq | Operator::Ne => SkipDecision::MustScan,
        _ => SkipDecision::UnsupportedForIndex,
    }
}

#[cfg(test)]
mod tests {
    use super::{SkipDecision, plan_chunk};
    use crate::engine::index::schema::IndexChunk;
    use crate::expr::parse_expression;

    fn chunk() -> IndexChunk {
        IndexChunk {
            ordinal: 0,
            first_record: 0,
            record_count: 10,
            chrom_start: "chr1".to_string(),
            chrom_end: "chr1".to_string(),
            pos_min: 1,
            pos_max: 10,
            qual_min: Some(1.0),
            qual_max: Some(20.0),
            filters: vec!["q10".to_string()],
            info_dp_min: Some(3),
            info_dp_max: Some(10),
            has_info_af: true,
            info_af_min: Some(0.01),
            info_af_max: Some(0.05),
            info_af_complete: true,
            format_keys: vec!["GT".to_string()],
            virtual_start: Some(0),
            virtual_end: Some(65_536),
        }
    }

    fn assert_plan(expression: &str, expected: SkipDecision) {
        let expression = parse_expression(expression).unwrap();

        assert_eq!(plan_chunk(&expression, &chunk()), expected);
    }

    fn assert_plan_with_chunk(expression: &str, chunk: IndexChunk, expected: SkipDecision) {
        let expression = parse_expression(expression).unwrap();

        assert_eq!(plan_chunk(&expression, &chunk), expected);
    }

    #[test]
    fn skips_chunk_when_qual_threshold_exceeds_chunk_max() {
        assert_plan("QUAL > 30", SkipDecision::CanSkip);
    }

    #[test]
    fn scans_chunk_when_qual_min_is_missing() {
        let mut chunk = chunk();
        chunk.qual_min = None;

        assert_plan_with_chunk("QUAL > 30", chunk, SkipDecision::MustScan);
    }

    #[test]
    fn scans_chunk_when_qual_max_is_missing() {
        let mut chunk = chunk();
        chunk.qual_max = None;

        assert_plan_with_chunk("QUAL > 30", chunk, SkipDecision::MustScan);
    }

    #[test]
    fn scans_chunk_when_filter_value_is_present() {
        assert_plan("FILTER == \"q10\"", SkipDecision::MustScan);
    }

    #[test]
    fn scans_chunk_for_filter_equality_under_tokenized_metadata() {
        assert_plan("FILTER == \"PASS\"", SkipDecision::MustScan);
        assert_plan("FILTER == \"missing\"", SkipDecision::MustScan);
    }

    #[test]
    fn scans_chunk_when_filter_not_equal_matches_missing_filter_values() {
        assert_plan("FILTER != \"q10\"", SkipDecision::MustScan);
    }

    #[test]
    fn skips_and_expression_when_either_side_cannot_match() {
        assert_plan("QUAL > 30 && FILTER == \"q10\"", SkipDecision::CanSkip);
    }

    #[test]
    fn skips_or_expression_when_both_sides_cannot_match() {
        assert_plan("QUAL > 30 || INFO/DP > 40", SkipDecision::CanSkip);
    }

    #[test]
    fn rejects_format_aggregate_for_index_planning() {
        assert_plan("ANY(FORMAT/AD > 80)", SkipDecision::UnsupportedForIndex);
    }

    #[test]
    fn scans_chunk_when_info_af_metadata_is_incomplete_and_bounds_are_absent() {
        let mut chunk = chunk();
        chunk.has_info_af = false;
        chunk.info_af_complete = false;
        chunk.info_af_min = None;
        chunk.info_af_max = None;

        assert_plan_with_chunk("INFO/AF > 0.2", chunk, SkipDecision::MustScan);
    }
}
