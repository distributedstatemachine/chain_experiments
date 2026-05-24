pub fn comma_record_fields<'a>(line: &'a str, prefix: &str, expected_len: usize) -> Vec<&'a str> {
    let record = line
        .trim()
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("record missing prefix {prefix:?}: {line}"));
    let fields = record.split(',').collect::<Vec<_>>();
    assert_eq!(
        fields.len(),
        expected_len,
        "unexpected field count for {prefix:?}: {line}"
    );
    assert!(
        fields
            .iter()
            .all(|field| !field.is_empty() && field.trim() == *field),
        "record has empty or whitespace-padded field for {prefix:?}: {line}"
    );
    fields
}

pub fn network_observation_root(line: &str) -> &str {
    comma_record_fields(line, "network_runtime_observation=", 13)[11]
}
