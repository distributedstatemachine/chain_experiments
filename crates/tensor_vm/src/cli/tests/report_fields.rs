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
