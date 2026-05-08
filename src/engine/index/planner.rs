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
        ExprNode::SampleAggregate { comparison, .. }
        | ExprNode::CountAggregate { comparison, .. } => {
            aggregate_format_decision(comparison, chunk)
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
        (Field::Chrom, Literal::String(value)) => {
            chrom_decision(&chunk.chrom_start, &chunk.chrom_end, comparison.op(), value)
        }
        (Field::Pos, Literal::Number(value)) => numeric_decision(
            Some(chunk.pos_min as f64),
            Some(chunk.pos_max as f64),
            comparison.op(),
            *value,
        ),
        (Field::Filter, Literal::String(value)) => {
            filter_decision(&chunk.filter_values, comparison.op(), value)
        }
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
        (Field::Info { key, index: None }, Literal::Number(value)) => {
            let Ok(key) = std::str::from_utf8(key) else {
                return SkipDecision::UnsupportedForIndex;
            };
            let Some(bounds) = chunk.info_numeric.get(key) else {
                return SkipDecision::CanSkip;
            };
            if !bounds.complete {
                SkipDecision::MustScan
            } else {
                numeric_decision(bounds.min, bounds.max, comparison.op(), *value)
            }
        }
        (Field::Info { .. }, _) => SkipDecision::UnsupportedForIndex,
        (Field::Format { key, .. }, _) => format_key_presence_decision(key, chunk),
        (Field::Chrom | Field::Pos | Field::Qual | Field::Filter, _) => {
            SkipDecision::UnsupportedForIndex
        }
    }
}

fn aggregate_format_decision(comparison: &Comparison, chunk: &IndexChunk) -> SkipDecision {
    match comparison.field() {
        Field::Format { key, .. } => format_key_presence_decision(key, chunk),
        _ => SkipDecision::UnsupportedForIndex,
    }
}

fn format_key_presence_decision(key: &[u8], chunk: &IndexChunk) -> SkipDecision {
    let Ok(key) = std::str::from_utf8(key) else {
        return SkipDecision::UnsupportedForIndex;
    };
    if chunk.format_keys.iter().any(|actual| actual == key) {
        SkipDecision::UnsupportedForIndex
    } else {
        SkipDecision::CanSkip
    }
}

fn chrom_decision(chrom_start: &str, chrom_end: &str, op: Operator, value: &str) -> SkipDecision {
    match op {
        Operator::Eq if chrom_start == chrom_end && chrom_start != value => SkipDecision::CanSkip,
        Operator::Ne if chrom_start == chrom_end && chrom_start == value => SkipDecision::CanSkip,
        Operator::Eq | Operator::Ne => SkipDecision::MustScan,
        _ => SkipDecision::UnsupportedForIndex,
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

fn filter_decision(filter_values: &[String], op: Operator, value: &str) -> SkipDecision {
    if filter_values.is_empty() {
        return SkipDecision::MustScan;
    }

    match op {
        Operator::Eq if !filter_values.iter().any(|actual| actual == value) => {
            SkipDecision::CanSkip
        }
        Operator::Ne if filter_values.len() == 1 && filter_values[0] == value => {
            SkipDecision::CanSkip
        }
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
            filter_values: vec!["q10".to_string()],
            info_dp_min: Some(3),
            info_dp_max: Some(10),
            has_info_af: true,
            info_af_min: Some(0.01),
            info_af_max: Some(0.05),
            info_af_complete: true,
            info_numeric: [
                (
                    "UNUSED7".to_string(),
                    crate::engine::index::schema::NumericBounds {
                        min: Some(100.0),
                        max: Some(200.0),
                        complete: true,
                    },
                ),
                (
                    "BROKEN".to_string(),
                    crate::engine::index::schema::NumericBounds {
                        min: None,
                        max: None,
                        complete: false,
                    },
                ),
            ]
            .into_iter()
            .collect(),
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
        assert_plan("FILTER == \"PASS\"", SkipDecision::CanSkip);
        assert_plan("FILTER == \"missing\"", SkipDecision::CanSkip);
    }

    #[test]
    fn scans_chunk_when_filter_not_equal_matches_missing_filter_values() {
        assert_plan("FILTER != \"q10\"", SkipDecision::CanSkip);
    }

    #[test]
    fn skips_chunk_when_chromosome_cannot_match() {
        assert_plan("CHROM == \"chr2\"", SkipDecision::CanSkip);
        assert_plan("CHROM != \"chr1\"", SkipDecision::CanSkip);
    }

    #[test]
    fn skips_chunk_when_position_range_cannot_match() {
        assert_plan("POS > 20", SkipDecision::CanSkip);
        assert_plan("POS < 1", SkipDecision::CanSkip);
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
    fn skips_format_predicates_when_key_is_absent_from_chunk() {
        assert_plan("ANY(FORMAT/AD > 80)", SkipDecision::CanSkip);
        assert_plan("N_PASS(FORMAT/AD > 80) > 0", SkipDecision::CanSkip);
        assert_plan("FORMAT/AD > 80", SkipDecision::CanSkip);
        assert_plan("FORMAT/GT == \"0/1\"", SkipDecision::UnsupportedForIndex);
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

    #[test]
    fn skips_arbitrary_info_numeric_when_chunk_bounds_cannot_match() {
        assert_plan("INFO/UNUSED7 > 300", SkipDecision::CanSkip);
        assert_plan("INFO/UNUSED7 < 50", SkipDecision::CanSkip);
        assert_plan("INFO/ABSENT > 1", SkipDecision::CanSkip);
    }

    #[test]
    fn scans_arbitrary_info_numeric_when_metadata_is_incomplete() {
        assert_plan("INFO/BROKEN > 1", SkipDecision::MustScan);
    }
}
