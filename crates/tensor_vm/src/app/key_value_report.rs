use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KeyValueReportError {
    DuplicateField,
    InvalidField,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyValueReport<'a> {
    fields: BTreeMap<&'a str, &'a str>,
}

#[derive(Default)]
pub(crate) struct KeyValueReportWriter {
    contents: String,
}

impl<'a> KeyValueReport<'a> {
    pub(crate) fn parse_strict(contents: &'a str) -> Result<Self, KeyValueReportError> {
        let mut fields = BTreeMap::new();
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            let (key, value) =
                parse_key_value_line(line).ok_or(KeyValueReportError::InvalidField)?;
            if fields.insert(key, value).is_some() {
                return Err(KeyValueReportError::DuplicateField);
            }
        }
        Ok(Self { fields })
    }

    pub(crate) fn parse_lenient(contents: &'a str) -> Self {
        let mut fields = BTreeMap::new();
        for line in contents.lines() {
            if let Some((key, value)) = parse_key_value_line(line) {
                fields.entry(key).or_insert(value);
            }
        }
        Self { fields }
    }

    pub(crate) fn value(&self, key: &str) -> Option<&'a str> {
        self.fields.get(key).copied()
    }

    pub(crate) fn into_owned(self) -> BTreeMap<String, String> {
        self.fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect()
    }
}

impl KeyValueReportWriter {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn field(&mut self, key: &str, value: impl std::fmt::Display) {
        let value = value.to_string();
        assert!(
            parse_key_value_line(&format!("{key}={value}")).is_some(),
            "invalid key-value report field {key:?}"
        );
        if !self.contents.is_empty() {
            self.contents.push('\n');
        }
        self.contents.push_str(key);
        self.contents.push('=');
        self.contents.push_str(&value);
    }

    #[cfg(test)]
    pub(crate) fn append_report(&mut self, report: &str) {
        let mut addition = String::new();
        for line in report.lines().filter(|line| !line.trim().is_empty()) {
            if !addition.is_empty() {
                addition.push('\n');
            }
            addition.push_str(line);
        }
        if addition.is_empty() {
            return;
        }
        KeyValueReport::parse_strict(&addition).expect("invalid key-value subreport");

        let mut combined = self.contents.clone();
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&addition);
        KeyValueReport::parse_strict(&combined).expect("duplicate key-value report field");
        self.contents = combined;
    }

    pub(crate) fn finish(self) -> String {
        self.contents
    }
}

fn parse_key_value_line(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    if key.is_empty() || key.trim() != key || value.is_empty() || value.trim() != value {
        return None;
    }
    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_value_report_parses_strict_fields_and_rejects_bad_lines() {
        let report = KeyValueReport::parse_strict("command=service_serve\np2p_runtime=libp2p\n")
            .expect("valid key-value report must parse");
        assert_eq!(report.value("command"), Some("service_serve"));
        assert_eq!(report.value("p2p_runtime"), Some("libp2p"));
        assert_eq!(
            KeyValueReport::parse_strict("command=service_serve\ncommand=service_status\n"),
            Err(KeyValueReportError::DuplicateField)
        );
        assert_eq!(
            KeyValueReport::parse_strict("command= service_serve\n"),
            Err(KeyValueReportError::InvalidField)
        );
        assert_eq!(
            KeyValueReport::parse_strict("not-a-field\n"),
            Err(KeyValueReportError::InvalidField)
        );
    }

    #[test]
    fn key_value_report_lenient_fields_keep_first_valid_value() {
        let fields = KeyValueReport::parse_lenient(
            "bad\ncommand=service_serve\ncommand=service_status\nempty=\nrole =miner\nrole=miner\n",
        )
        .into_owned();
        assert_eq!(
            fields.get("command").map(String::as_str),
            Some("service_serve")
        );
        assert_eq!(fields.get("role").map(String::as_str), Some("miner"));
        assert!(!fields.contains_key("empty"));
    }

    #[test]
    fn key_value_report_writer_renders_parseable_fields() {
        let mut report = KeyValueReportWriter::new();
        report.field("command", "service_serve");
        report.field("max_requests", 7);
        let report = report.finish();

        assert_eq!(report, "command=service_serve\nmax_requests=7");
        let parsed = KeyValueReport::parse_strict(&report).expect("writer output must parse");
        assert_eq!(parsed.value("command"), Some("service_serve"));
        assert_eq!(parsed.value("max_requests"), Some("7"));
    }

    #[test]
    fn key_value_report_writer_appends_parseable_subreports() {
        let mut report = KeyValueReportWriter::new();
        report.field("command", "service_serve");
        report.append_report("p2p_runtime=libp2p\np2p_identity_seeded=false\n");
        let report = report.finish();

        assert_eq!(
            report,
            "command=service_serve\np2p_runtime=libp2p\np2p_identity_seeded=false"
        );
        let parsed = KeyValueReport::parse_strict(&report).expect("writer output must parse");
        assert_eq!(parsed.value("p2p_runtime"), Some("libp2p"));
        assert_eq!(parsed.value("p2p_identity_seeded"), Some("false"));
    }

    #[test]
    #[should_panic(expected = "duplicate key-value report field")]
    fn key_value_report_writer_rejects_duplicate_appended_fields() {
        let mut report = KeyValueReportWriter::new();
        report.field("command", "service_serve");
        report.append_report("command=service_status");
    }
}
