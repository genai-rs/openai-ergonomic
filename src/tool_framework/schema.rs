//! JSON Schema clean-up for tools only known at runtime (MCP servers, plugins), whose schemas
//! are not written for the `OpenAI` API.
//!
//! [`sanitize`] rewrites what the API rejects; [`compact`] shrinks a large schema step by step.
//! Both are pure and change the schema in place;
//! [`ToolRegistry::register_dynamic`](super::ToolRegistry::register_dynamic) applies them.
//!
//! ```
//! use openai_ergonomic::tool_framework::schema;
//! use serde_json::json;
//!
//! let mut value = json!({"properties": {"mode": {"const": "fast"}, "any": true}});
//! let report = schema::sanitize(&mut value);
//! assert!(report.changed());
//! assert_eq!(value, json!({
//!     "type": "object",
//!     "properties": {
//!         "mode": {"type": "string", "enum": ["fast"]},
//!         "any": {"type": "string"}
//!     }
//! }));
//! ```

use std::collections::BTreeSet;

use serde_json::{json, Map, Value};

/// What [`sanitize`] changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SanitizeReport {
    /// `true`/`false` schemas replaced with `{"type": "string"}`.
    pub booleans_replaced: usize,
    /// `const` values turned into a one-value `enum`.
    pub consts_converted: usize,
    /// Schemas without a `type` that got one.
    pub types_inferred: usize,
    /// Object schemas without `properties` that got an empty one.
    pub properties_added: usize,
    /// Definitions (`$defs`, `definitions`) no `$ref` reaches, removed.
    pub definitions_pruned: usize,
}

impl SanitizeReport {
    /// Whether anything changed.
    #[must_use]
    pub fn changed(&self) -> bool {
        *self != Self::default()
    }
}

/// Rewrite `schema` into one the API accepts as tool parameters:
///
/// - a `true`/`false` schema becomes `{"type": "string"}`;
/// - `const` becomes a one-value `enum`;
/// - a missing `type` is inferred: `object` with `properties` (and at the root), `array` with
///   `items`, the type of the `enum` values, else `string` (unless `anyOf`/`oneOf`/`allOf`/`$ref`
///   describe it);
/// - an object without `properties` gets an empty one;
/// - definitions that no `$ref` reaches are removed.
///
/// Applying it twice changes nothing more.
pub fn sanitize(schema: &mut Value) -> SanitizeReport {
    let mut report = SanitizeReport::default();
    if !schema.is_object() {
        *schema = json!({});
    }
    if let Some(map) = schema.as_object_mut() {
        if !map.contains_key("type") && !has_composition(map) {
            map.insert("type".into(), "object".into());
            report.types_inferred += 1;
        }
    }
    sanitize_node(schema, &mut report);
    report.definitions_pruned = prune_definitions(schema);
    report
}

fn has_composition(map: &Map<String, Value>) -> bool {
    ["anyOf", "oneOf", "allOf", "$ref"]
        .iter()
        .any(|key| map.contains_key(*key))
}

fn sanitize_node(node: &mut Value, report: &mut SanitizeReport) {
    if node.is_boolean() {
        *node = json!({"type": "string"});
        report.booleans_replaced += 1;
        return;
    }
    let Some(map) = node.as_object_mut() else {
        return;
    };
    if let Some(value) = map.remove("const") {
        map.insert("enum".into(), Value::Array(vec![value]));
        report.consts_converted += 1;
    }
    if !map.contains_key("type") && !has_composition(map) {
        let inferred = if map.contains_key("properties") {
            "object"
        } else if map.contains_key("items") {
            "array"
        } else {
            map.get("enum")
                .and_then(Value::as_array)
                .and_then(|values| values.iter().find(|value| !value.is_null()))
                .map_or("string", json_type)
        };
        map.insert("type".into(), inferred.into());
        report.types_inferred += 1;
    }
    let is_object = match map.get("type") {
        Some(Value::String(kind)) => kind == "object",
        Some(Value::Array(kinds)) => kinds.iter().any(|kind| kind == "object"),
        _ => false,
    };
    if is_object && !map.contains_key("properties") {
        map.insert("properties".into(), json!({}));
        report.properties_added += 1;
    }
    for_each_subschema(map, |child| sanitize_node(child, report));
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "boolean",
        Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::String(_) | Value::Null => "string",
    }
}

/// Call `f` on every direct subschema of a schema object.
fn for_each_subschema(map: &mut Map<String, Value>, mut f: impl FnMut(&mut Value)) {
    for key in ["properties", "patternProperties", "$defs", "definitions"] {
        if let Some(Value::Object(children)) = map.get_mut(key) {
            children.values_mut().for_each(&mut f);
        }
    }
    for key in ["items", "not", "contains"] {
        match map.get_mut(key) {
            Some(Value::Array(children)) => children.iter_mut().for_each(&mut f),
            Some(child @ (Value::Object(_) | Value::Bool(_))) => f(child),
            _ => {}
        }
    }
    // `additionalProperties: false` is a valid keyword value, not a schema to replace.
    if let Some(child @ Value::Object(_)) = map.get_mut("additionalProperties") {
        f(child);
    }
    for key in ["anyOf", "oneOf", "allOf", "prefixItems"] {
        if let Some(Value::Array(children)) = map.get_mut(key) {
            children.iter_mut().for_each(&mut f);
        }
    }
}

/// Remove the root definitions no `$ref` reaches; returns how many.
fn prune_definitions(schema: &mut Value) -> usize {
    let Some(root) = schema.as_object() else {
        return 0;
    };
    let mut reachable = BTreeSet::new();
    let mut pending = Vec::new();
    // References outside the definitions.
    let mut outside = root.clone();
    outside.remove("$defs");
    outside.remove("definitions");
    collect_refs(&Value::Object(outside), &mut pending);
    while let Some(reference) = pending.pop() {
        if !reachable.insert(reference.clone()) {
            continue;
        }
        if let Some(target) = definition(root, &reference) {
            collect_refs(target, &mut pending);
        }
    }
    let mut pruned = 0;
    let Some(root) = schema.as_object_mut() else {
        return 0;
    };
    for key in ["$defs", "definitions"] {
        if let Some(Value::Object(definitions)) = root.get_mut(key) {
            let before = definitions.len();
            definitions.retain(|name, _| reachable.contains(&format!("#/{key}/{name}")));
            pruned += before - definitions.len();
            if definitions.is_empty() {
                root.remove(key);
            }
        }
    }
    pruned
}

fn definition<'a>(root: &'a Map<String, Value>, reference: &str) -> Option<&'a Value> {
    let path = reference.strip_prefix("#/")?;
    let (key, name) = path.split_once('/')?;
    root.get(key)?.as_object()?.get(name)
}

fn collect_refs(node: &Value, out: &mut Vec<String>) {
    match node {
        Value::Object(map) => {
            for (key, value) in map {
                match (key.as_str(), value) {
                    ("$ref", Value::String(reference)) => out.push(reference.clone()),
                    _ => collect_refs(value, out),
                }
            }
        }
        Value::Array(values) => values.iter().for_each(|value| collect_refs(value, out)),
        _ => {}
    }
}

/// What [`compact`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompactReport {
    /// The size of the schema before, in bytes of compact JSON.
    pub original_bytes: usize,
    /// The size after.
    pub final_bytes: usize,
    /// How many steps were applied (0 to 4), in order: descriptions stripped, definitions
    /// dropped, schemas below depth 3 collapsed, compositions pruned.
    pub steps: usize,
}

impl CompactReport {
    /// Whether the schema now fits the limit it was compacted for.
    #[must_use]
    pub fn fits(&self, max_bytes: usize) -> bool {
        self.final_bytes <= max_bytes
    }
}

/// The size of `schema` in bytes of compact JSON.
#[must_use]
pub fn size(schema: &Value) -> usize {
    serde_json::to_string(schema).map_or(0, |text| text.len())
}

/// Shrink `schema` until it is at most `max_bytes` (compact JSON), applying these steps in order
/// and stopping as soon as it fits:
///
/// 1. strip the `description`s (and `title`s, `examples`);
/// 2. drop the definitions, every `$ref` becoming `{}` (any value);
/// 3. collapse the schemas nested deeper than 3 levels to their `type`;
/// 4. replace each `anyOf`/`oneOf` with its first alternative and merge `allOf`.
///
/// It may still not fit: check [`CompactReport::fits`].
///
/// ```
/// use openai_ergonomic::tool_framework::schema;
/// use serde_json::json;
///
/// let mut value = json!({"type": "object", "properties": {
///     "query": {"type": "string", "description": "A long description of the query"}
/// }});
/// let report = schema::compact(&mut value, 60);
/// assert!(report.fits(60));
/// assert_eq!(value, json!({"type": "object", "properties": {"query": {"type": "string"}}}));
/// ```
pub fn compact(schema: &mut Value, max_bytes: usize) -> CompactReport {
    let original_bytes = size(schema);
    let mut report = CompactReport {
        original_bytes,
        final_bytes: original_bytes,
        steps: 0,
    };
    let steps: [fn(&mut Value); 4] = [
        strip_descriptions,
        drop_definitions,
        |schema| collapse_below(schema, 0),
        prune_compositions,
    ];
    for step in steps {
        if report.final_bytes <= max_bytes {
            break;
        }
        step(schema);
        report.steps += 1;
        report.final_bytes = size(schema);
    }
    report
}

fn strip_descriptions(node: &mut Value) {
    if let Some(map) = node.as_object_mut() {
        for key in ["description", "title", "examples"] {
            map.remove(key);
        }
        for_each_subschema(map, strip_descriptions);
    }
}

fn drop_definitions(node: &mut Value) {
    if let Some(map) = node.as_object_mut() {
        map.remove("$defs");
        map.remove("definitions");
    }
    replace_refs(node);
}

fn replace_refs(node: &mut Value) {
    if let Some(map) = node.as_object_mut() {
        if map.contains_key("$ref") {
            *node = json!({});
            return;
        }
        for_each_subschema(map, replace_refs);
    }
}

const MAX_DEPTH: usize = 3;

fn collapse_below(node: &mut Value, depth: usize) {
    let Some(map) = node.as_object_mut() else {
        return;
    };
    if depth >= MAX_DEPTH {
        let kind = map.get("type").cloned();
        map.clear();
        if let Some(kind) = kind {
            map.insert("type".into(), kind);
        }
        return;
    }
    for_each_subschema(map, |child| collapse_below(child, depth + 1));
}

fn prune_compositions(node: &mut Value) {
    let Some(map) = node.as_object_mut() else {
        return;
    };
    for key in ["anyOf", "oneOf"] {
        if let Some(Value::Array(alternatives)) = map.remove(key) {
            if let Some(Value::Object(first)) = alternatives
                .into_iter()
                .find(|alternative| alternative.get("type").and_then(Value::as_str) != Some("null"))
            {
                for (field, value) in first {
                    map.entry(field).or_insert(value);
                }
            }
        }
    }
    if let Some(Value::Array(parts)) = map.remove("allOf") {
        for part in parts {
            if let Value::Object(part) = part {
                for (field, value) in part {
                    map.entry(field).or_insert(value);
                }
            }
        }
    }
    for_each_subschema(map, prune_compositions);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn sanitize_infers_types_and_prunes_definitions() {
        let mut value = json!({
            "properties": {
                "count": {"enum": [1, 2]},
                "tags": {"items": {"const": 3.5}},
                "ref": {"$ref": "#/$defs/Used"},
                "nested": {"type": "object"}
            },
            "$defs": {
                "Used": {"properties": {"inner": {"$ref": "#/$defs/Inner"}}},
                "Inner": {"type": "string"},
                "Unused": {"type": "string"}
            }
        });
        let report = sanitize(&mut value);
        assert_eq!(value["type"], "object");
        assert_eq!(value["properties"]["count"]["type"], "integer");
        assert_eq!(value["properties"]["tags"]["type"], "array");
        assert_eq!(
            value["properties"]["tags"]["items"],
            json!({"type": "number", "enum": [3.5]})
        );
        assert_eq!(value["properties"]["ref"], json!({"$ref": "#/$defs/Used"}));
        assert_eq!(value["properties"]["nested"]["properties"], json!({}));
        assert_eq!(value["$defs"]["Used"]["type"], "object");
        assert!(value["$defs"].get("Unused").is_none());
        assert!(value["$defs"].get("Inner").is_some());
        assert_eq!(report.definitions_pruned, 1);
        assert_eq!(report.consts_converted, 1);

        let before = value.clone();
        assert!(!sanitize(&mut value).changed());
        assert_eq!(value, before);

        let mut value = json!(true);
        sanitize(&mut value);
        assert_eq!(value, json!({"type": "object", "properties": {}}));
    }

    #[test]
    fn a_property_named_description_survives_compaction() {
        let mut value = json!({"type": "object", "description": "drop me", "properties": {
            "description": {"type": "string", "description": "drop me too"}
        }});
        compact(&mut value, 10);
        assert_eq!(
            value["properties"]["description"],
            json!({"type": "string"})
        );
        assert!(value.get("description").is_none());
    }

    #[test]
    fn compaction_goes_step_by_step() {
        let deep = json!({"type": "object", "properties": {"a": {"type": "object", "properties": {
            "b": {"type": "object", "properties": {"c": {"type": "object", "properties": {
                "d": {"type": "string"}}}}}}}}});
        let mut value = deep.clone();
        let report = compact(&mut value, size(&deep));
        assert_eq!(report.steps, 0);
        let report = compact(&mut value, 130);
        assert_eq!(report.steps, 3);
        assert_eq!(
            value["properties"]["a"]["properties"]["b"]["properties"]["c"],
            json!({"type": "object"})
        );

        let mut value = json!({"type": "object", "properties": {"x": {"anyOf": [
            {"type": "null"}, {"type": "integer", "minimum": 0}, {"type": "string"}
        ]}, "y": {"$ref": "#/$defs/Y"}}, "$defs": {"Y": {"type": "string"}}});
        let report = compact(&mut value, 0);
        assert_eq!(report.steps, 4);
        assert!(!report.fits(0));
        assert_eq!(
            value["properties"]["x"],
            json!({"type": "integer", "minimum": 0})
        );
        assert_eq!(value["properties"]["y"], json!({}));
    }
}
