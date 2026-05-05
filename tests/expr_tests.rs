use vcf_fast::expr::{EvalRecord, FormatValues, parse_expression};

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
        format: FormatValues::default(),
    }
}

fn record_with_format<'a>(format: FormatValues<'a>) -> EvalRecord<'a> {
    EvalRecord {
        chrom: "1",
        pos: 100,
        qual: Some(50.0),
        filter: "PASS",
        info: "DP=10;AF=0.1",
        format,
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

#[test]
fn evaluates_format_numeric_predicates() {
    let expr = parse_expression("FORMAT/DP > 20 && FORMAT/GQ >= 30").unwrap();

    assert!(expr.evaluate(&record_with_format(FormatValues {
        gt: Some("0/1"),
        dp: Some("25"),
        gq: Some("40"),
    })));
    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: Some("0/1"),
        dp: Some("10"),
        gq: Some("40"),
    })));
}

#[test]
fn evaluates_format_gt_as_exact_string() {
    let expr = parse_expression("FORMAT/GT == \"0/1\"").unwrap();

    assert!(expr.evaluate(&record_with_format(FormatValues {
        gt: Some("0/1"),
        dp: None,
        gq: None,
    })));
    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: Some("0|1"),
        dp: None,
        gq: None,
    })));

    let missing_dot = parse_expression("FORMAT/GT == \".\"").unwrap();
    assert!(!missing_dot.evaluate(&record_with_format(FormatValues {
        gt: Some("."),
        dp: None,
        gq: None,
    })));

    let missing_empty = parse_expression("FORMAT/GT == \"\"").unwrap();
    assert!(!missing_empty.evaluate(&record_with_format(FormatValues {
        gt: Some(""),
        dp: None,
        gq: None,
    })));
}

#[test]
fn format_missing_or_invalid_numeric_values_are_false() {
    let expr = parse_expression("FORMAT/DP > 20").unwrap();

    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: None,
        dp: None,
        gq: None,
    })));
    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: None,
        dp: Some("."),
        gq: None,
    })));
    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: None,
        dp: Some("not-a-number"),
        gq: None,
    })));
}

#[test]
fn rejects_bare_gq_and_requires_explicit_format_prefix() {
    let error = parse_expression("GQ > 20").unwrap_err().to_string();

    assert!(error.contains("unsupported field 'GQ'"));
}

#[test]
fn parses_and_evaluates_arbitrary_info_numeric_field() {
    let expr = parse_expression("INFO/MQ >= 50").unwrap();
    let record = EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS")
        .with_info(b"MQ=60;CSQ=synonymous_variant");

    assert!(expr.evaluate_record(&record));
}

#[test]
fn arbitrary_info_numeric_uses_any_comma_value_semantics() {
    let expr = parse_expression("INFO/FS < 10").unwrap();
    let record =
        EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS").with_info(b"FS=12.5,8.2,30.0");

    assert!(expr.evaluate_record(&record));
}

#[test]
fn arbitrary_info_string_compares_raw_value() {
    let expr = parse_expression("INFO/CSQ == \"synonymous_variant\"").unwrap();
    let record = EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS")
        .with_info(b"MQ=60;CSQ=synonymous_variant");

    assert!(expr.evaluate_record(&record));
}

#[test]
fn arbitrary_info_missing_empty_flag_and_dot_are_false() {
    let missing = parse_expression("INFO/MQ >= 50").unwrap();
    let empty = parse_expression("INFO/EMPTY == \"value\"").unwrap();
    let flag = parse_expression("INFO/SOMATIC == \"true\"").unwrap();
    let dot = parse_expression("INFO/AF > 0.01").unwrap();
    let record =
        EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS").with_info(b"EMPTY=;SOMATIC;AF=.");

    assert!(!missing.evaluate_record(&record));
    assert!(!empty.evaluate_record(&record));
    assert!(!flag.evaluate_record(&record));
    assert!(!dot.evaluate_record(&record));
}
