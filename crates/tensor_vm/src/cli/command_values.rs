use super::arguments::parse_hash_argument;
use crate::types::Hash;

pub(super) fn parse_hash_value(value: &str) -> std::result::Result<Hash, String> {
    parse_hash_argument(value).map_err(|error| error.to_string())
}
