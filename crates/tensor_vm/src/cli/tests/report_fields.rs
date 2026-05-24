use super::super::evidence_fields::exact_comma_fields;
use crate::app::{KeyValueReport, KeyValueReportError};

pub(super) fn assert_report_fields(report: &str, expected: &[(&str, &str)]) {
    let fields = parse_key_value_report(report);
    for (key, expected_value) in expected {
        assert_eq!(
            fields.value(key),
            Some(*expected_value),
            "unexpected report field {key:?}\nreport:\n{report}"
        );
    }
}

fn parse_key_value_report(report: &str) -> KeyValueReport<'_> {
    KeyValueReport::parse_strict(report).unwrap_or_else(|error| match error {
        KeyValueReportError::DuplicateField => {
            panic!("duplicate report field\nreport:\n{report}")
        }
        KeyValueReportError::InvalidField => {
            panic!("report line is not key=value\nreport:\n{report}")
        }
    })
}

pub(super) fn comma_record_fields<'a>(
    line: &'a str,
    prefix: &str,
    expected_len: usize,
) -> Vec<&'a str> {
    let record = line
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("record missing prefix {prefix:?}: {line}"));
    exact_comma_fields(record, expected_len, "invalid comma record")
        .unwrap_or_else(|error| panic!("unexpected comma record for {prefix:?}: {line}: {error}"))
}
