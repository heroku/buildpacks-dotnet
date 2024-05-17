/// Convert a [`libcnb::Env`] to a sorted vector of key-value string slice tuples, for easier
/// testing of the environment variables set in the buildpack layers.
#[cfg(test)]
pub(crate) fn environment_as_sorted_vector(environment: &libcnb::Env) -> Vec<(&str, &str)> {
    let mut result: Vec<(&str, &str)> = environment
        .iter()
        .map(|(k, v)| (k.to_str().unwrap(), v.to_str().unwrap()))
        .collect();

    result.sort_by_key(|kv| kv.0);
    result
}
