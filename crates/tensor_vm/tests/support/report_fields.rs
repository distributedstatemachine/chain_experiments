use std::collections::BTreeMap;

pub fn report_value<'a>(report: &'a str, key: &str) -> &'a str {
    report_values(report, key)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("expected report field {key}"))
}

pub fn report_values<'a>(report: &'a str, key: &str) -> Vec<&'a str> {
    parse_report(report).remove(key).unwrap_or_default()
}

pub fn report_u64(report: &str, key: &str) -> u64 {
    report_value(report, key)
        .parse()
        .unwrap_or_else(|_| panic!("expected numeric report field {key}"))
}

fn parse_report(report: &str) -> BTreeMap<&str, Vec<&str>> {
    let mut fields = BTreeMap::new();
    for (index, line) in report.lines().enumerate() {
        let (key, value) = line.split_once('=').unwrap_or_else(|| {
            panic!(
                "report line {} is not key=value: {line:?}\nreport:\n{report}",
                index + 1
            )
        });
        assert!(!key.is_empty(), "empty report key\nreport:\n{report}");
        fields.entry(key).or_insert_with(Vec::new).push(value);
    }
    fields
}
