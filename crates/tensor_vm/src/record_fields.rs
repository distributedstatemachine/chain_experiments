pub(crate) fn exact_comma_fields(value: &str, expected_len: usize) -> Option<Vec<&str>> {
    let fields = value.split(',').collect::<Vec<_>>();
    if fields.len() != expected_len
        || fields
            .iter()
            .any(|field| field.is_empty() || field.trim() != *field)
    {
        return None;
    }
    Some(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_comma_fields_rejects_padded_empty_and_wrong_count_records() {
        assert_eq!(
            exact_comma_fields("alpha,beta", 2),
            Some(vec!["alpha", "beta"])
        );
        assert_eq!(exact_comma_fields("alpha,beta,gamma", 2), None);
        assert_eq!(exact_comma_fields("alpha,", 2), None);
        assert_eq!(exact_comma_fields("alpha, beta", 2), None);
        assert_eq!(exact_comma_fields(" alpha,beta", 2), None);
    }
}
