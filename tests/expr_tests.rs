use vcf_fast::expr::{EvalRecord, parse_expression};

fn record<'a>(
    chrom: &'a str,
    pos: u64,
    qual: Option<f64>,
    filter: &'a str,
    info: &'a str,
) -> EvalRecord<'a> {
    EvalRecord {
        chrom,
        pos,
        qual,
        filter,
        info,
    }
}

#[test]
fn parses_and_evaluates_numeric_comparisons() {
    let expr = parse_expression("QUAL >= 30 && DP > 10").unwrap();

    assert!(expr.evaluate(&record("1", 200, Some(35.0), "PASS", "DP=12;AF=0.03")));
    assert!(!expr.evaluate(&record("1", 100, Some(10.0), "PASS", "DP=20;AF=0.02")));
}

#[test]
fn supports_explicit_info_field_names() {
    let expr = parse_expression("INFO/DP > 10 && INFO/AF > 0.01").unwrap();

    assert!(expr.evaluate(&record("1", 200, Some(35.0), "PASS", "DP=12;AF=0.03")));
    assert!(!expr.evaluate(&record("1", 100, Some(35.0), "PASS", "DP=8;AF=0.03")));
}

#[test]
fn supports_or_with_and_precedence() {
    let expr = parse_expression("QUAL > 50 || DP > 10 && FILTER == \"PASS\"").unwrap();

    assert!(expr.evaluate(&record("1", 100, Some(20.0), "PASS", "DP=12")));
    assert!(expr.evaluate(&record("1", 100, Some(60.0), "q10", "DP=1")));
    assert!(!expr.evaluate(&record("1", 100, Some(20.0), "q10", "DP=12")));
}

#[test]
fn supports_parentheses_for_boolean_grouping() {
    let expr = parse_expression("(QUAL > 50 || DP > 10) && FILTER == \"PASS\"").unwrap();

    assert!(expr.evaluate(&record("1", 100, Some(20.0), "PASS", "DP=12")));
    assert!(!expr.evaluate(&record("1", 100, Some(60.0), "q10", "DP=1")));
}

#[test]
fn parses_and_evaluates_string_comparisons() {
    let expr = parse_expression("CHROM == \"1\" && FILTER != \"q10\"").unwrap();

    assert!(expr.evaluate(&record("1", 200, Some(35.0), "PASS", "DP=12")));
    assert!(!expr.evaluate(&record("2", 400, Some(50.0), "q10", "DP=5")));
}

#[test]
fn missing_numeric_values_make_predicate_false() {
    let missing_qual = parse_expression("QUAL > 30").unwrap();
    let missing_info = parse_expression("AF > 0.01").unwrap();

    assert!(!missing_qual.evaluate(&record("1", 300, None, "PASS", "DP=50")));
    assert!(!missing_info.evaluate(&record("1", 300, Some(40.0), "PASS", "DP=50")));
}

#[test]
fn comma_separated_numeric_info_values_pass_when_any_value_matches() {
    let expr = parse_expression("AF > 0.01").unwrap();

    assert!(expr.evaluate(&record("2", 500, Some(60.0), "PASS", "DP=22;AF=0.005,0.02")));
}

#[test]
fn rejects_malformed_expressions() {
    let err = parse_expression("QUAL >").unwrap_err().to_string();

    assert!(err.contains("expected literal"));
}
