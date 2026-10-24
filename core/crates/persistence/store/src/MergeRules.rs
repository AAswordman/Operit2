//! Shares content comparison and deterministic names across preference and collection storage.
use std::collections::{BTreeMap, BTreeSet};
use serde_json::{Map, Value};

/// Compares complete JSON content after excluding the declared identity and metadata fields.
pub(crate) fn canonicalContent<'a>(
    value: &Value,
    identityField: &str,
    ignoredFields: impl IntoIterator<Item = &'a str>,
) -> Result<String, String> {
    let mut object = value.as_object().ok_or("Merge entity must be a JSON object")?.clone();
    object.remove(identityField);
    for field in ignoredFields { object.remove(field); }
    serde_json::to_string(&canonicalValue(Value::Object(object))).map_err(|error| error.to_string())
}

/// Produces stable nested JSON content while retaining array order as business data.
fn canonicalValue(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(object.into_iter()
            .map(|(key, value)| (key, canonicalValue(value)))
            .collect::<BTreeMap<_, _>>().into_iter().collect::<Map<_, _>>()),
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalValue).collect()),
        scalar => scalar,
    }
}

/// Assigns collision-free display names in identity order while reserving every requested name.
pub(crate) fn uniqueNames(names: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let reserved = names.values().collect::<BTreeSet<_>>();
    let mut assigned = BTreeSet::new();
    let mut result = BTreeMap::new();
    for (identity, base) in names {
        let mut name = base.clone();
        if assigned.contains(&name) {
            let mut suffix = 1usize;
            loop {
                name = format!("{base}（{suffix}）");
                if !reserved.contains(&name) && !assigned.contains(&name) { break; }
                suffix += 1;
            }
        }
        assigned.insert(name.clone());
        result.insert(identity.clone(), name);
    }
    result
}
