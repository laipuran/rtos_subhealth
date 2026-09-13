use diagnosis::schema::{parse_diagnosis, passes_confidence, SchemaError};

const GOOD: &str = r#"{"severity":"mild","summary":"略低","possible_causes":["疲劳"],
"recommendations":["休息"],"confidence":0.9,"disclaimer":"仅供参考"}"#;

#[test]
fn parses_plain_json() {
    let d = parse_diagnosis(GOOD).unwrap();
    assert_eq!(d.severity, "mild");
    assert_eq!(d.possible_causes, vec!["疲劳"]);
    assert_eq!(d.confidence, 0.9);
}

#[test]
fn strips_json_fences() {
    let fenced = format!("```json\n{GOOD}\n```");
    assert_eq!(parse_diagnosis(&fenced).unwrap().severity, "mild");
}

#[test]
fn salvages_first_object_from_prose() {
    let text = format!("here you go: {GOOD} thanks");
    assert_eq!(parse_diagnosis(&text).unwrap().severity, "mild");
}

#[test]
fn missing_fields_is_error() {
    let err = parse_diagnosis(r#"{"severity":"mild"}"#).unwrap_err();
    match err {
        SchemaError::Missing(fields) => assert!(fields.contains(&"summary".to_string())),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn invalid_severity_is_error() {
    let bad = GOOD.replace("mild", "catastrophic");
    assert!(matches!(
        parse_diagnosis(&bad),
        Err(SchemaError::InvalidSeverity(_))
    ));
}

#[test]
fn non_array_lists_are_wrapped() {
    let value = r#"{"severity":"mild","summary":"s","possible_causes":"疲劳",
        "recommendations":"休息","confidence":0.5,"disclaimer":"d"}"#;
    let d = parse_diagnosis(value).unwrap();
    assert_eq!(d.possible_causes, vec!["疲劳"]);
    assert_eq!(d.recommendations, vec!["休息"]);
}

#[test]
fn confidence_threshold_filter() {
    let d = parse_diagnosis(GOOD).unwrap();
    assert!(passes_confidence(&d, 0.8));
    assert!(!passes_confidence(&d, 0.95));
}
