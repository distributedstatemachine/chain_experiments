use std::collections::BTreeMap;

pub(super) fn assert_report_fields(report: &str, expected: &[(&str, &str)]) {
    let fields = parse_key_value_report(report);
    for (key, expected_value) in expected {
        assert_eq!(
            fields.get(key).copied(),
            Some(*expected_value),
            "unexpected report field {key:?}\nreport:\n{report}"
        );
    }
}

fn parse_key_value_report(report: &str) -> BTreeMap<&str, &str> {
    let mut fields = BTreeMap::new();
    for (index, line) in report.lines().enumerate() {
        let (key, value) = line.split_once('=').unwrap_or_else(|| {
            panic!(
                "report line {} is not key=value: {line:?}\nreport:\n{report}",
                index + 1
            )
        });
        assert!(!key.is_empty(), "empty report key\nreport:\n{report}");
        assert!(
            fields.insert(key, value).is_none(),
            "duplicate report field {key:?}\nreport:\n{report}"
        );
    }
    fields
}
