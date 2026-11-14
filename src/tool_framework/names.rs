//! Tool names for tools only known at runtime (MCP servers, plugins).

use std::collections::BTreeSet;

/// A tool name the API accepts, from any text.
///
/// Every character other than an ASCII letter, a digit or `_` becomes `_`, and an empty result
/// becomes `"_"`. The length is not limited here (see [`ToolNamer`]).
///
/// ```
/// use openai_ergonomic::tool_framework::sanitize_tool_name;
///
/// assert_eq!(sanitize_tool_name("github.create-issue"), "github_create_issue");
/// assert_eq!(sanitize_tool_name(""), "_");
/// ```
#[must_use]
pub fn sanitize_tool_name(raw: &str) -> String {
    let name: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() {
        "_".to_string()
    } else {
        name
    }
}

/// Gives each runtime tool a unique, valid name.
///
/// The name is `namespace__name`, sanitized ([`sanitize_tool_name`]), at most `max_len`
/// characters. A name that is too long, or already given, keeps a prefix and ends with `_` and
/// 12 hex digits of a hash of the raw name, so it stays the same from one run to the next.
///
/// ```
/// use openai_ergonomic::tool_framework::ToolNamer;
///
/// let mut namer = ToolNamer::chat();
/// assert_eq!(namer.unique(Some("github"), "create-issue"), "github__create_issue");
/// let again = namer.unique(Some("github"), "create_issue");
/// assert_ne!(again, "github__create_issue");
/// assert!(again.starts_with("github__create_issue_"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolNamer {
    max_len: usize,
    taken: BTreeSet<String>,
}

/// The maximum length of a Chat Completions tool name, and of a name in a [`ToolRegistry`](super::ToolRegistry).
pub const CHAT_TOOL_NAME_MAX: usize = 64;
/// The maximum length of a Responses API tool name.
pub const RESPONSES_TOOL_NAME_MAX: usize = 128;
const HASH_LEN: usize = 12;

impl ToolNamer {
    /// A namer for names of at most `max_len` characters (at least 14, room for the hash).
    #[must_use]
    pub fn new(max_len: usize) -> Self {
        Self {
            max_len: max_len.max(HASH_LEN + 2),
            taken: BTreeSet::new(),
        }
    }

    /// A namer for Chat Completions (and [`ToolRegistry`](super::ToolRegistry)) names: 64 characters.
    #[must_use]
    pub fn chat() -> Self {
        Self::new(CHAT_TOOL_NAME_MAX)
    }

    /// A namer for Responses API names: 128 characters.
    #[must_use]
    pub fn responses() -> Self {
        Self::new(RESPONSES_TOOL_NAME_MAX)
    }

    /// Mark `name` as given, so that [`unique`](Self::unique) does not give it again.
    pub fn reserve(&mut self, name: impl Into<String>) {
        self.taken.insert(name.into());
    }

    /// A unique name for the tool `name` of `namespace`.
    pub fn unique(&mut self, namespace: Option<&str>, name: &str) -> String {
        let name = unique_name(namespace, name, self.max_len, |candidate| {
            self.taken.contains(candidate)
        });
        self.taken.insert(name.clone());
        name
    }
}

/// A sanitized name for `namespace__name` of at most `max_len` characters that `taken` does not
/// report as given.
pub fn unique_name(
    namespace: Option<&str>,
    name: &str,
    max_len: usize,
    taken: impl Fn(&str) -> bool,
) -> String {
    let raw = namespace.map_or_else(
        || name.to_string(),
        |namespace| format!("{namespace}__{name}"),
    );
    let sanitized = sanitize_tool_name(&raw);
    if sanitized.len() <= max_len && !taken(&sanitized) {
        return sanitized;
    }
    let prefix_len = max_len.saturating_sub(HASH_LEN + 1);
    let prefix = &sanitized[..sanitized.len().min(prefix_len)];
    let mut seed = raw;
    loop {
        let candidate = format!(
            "{prefix}_{:012x}",
            fnv1a(seed.as_bytes()) & 0xffff_ffff_ffff
        );
        if !taken(&candidate) {
            return candidate;
        }
        seed.push('\'');
    }
}

/// The 64-bit FNV-1a hash: stable across runs and platforms.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_names_are_hashed_and_stable() {
        let long = "x".repeat(100);
        let first = ToolNamer::chat().unique(Some("server"), &long);
        let second = ToolNamer::chat().unique(Some("server"), &long);
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.starts_with("server__xxx"));
        assert!(ToolNamer::responses().unique(Some("server"), &long).len() <= 128);
    }

    #[test]
    fn collisions_get_distinct_names() {
        let mut namer = ToolNamer::new(20);
        let a = namer.unique(None, "a-b");
        let b = namer.unique(None, "a.b");
        let c = namer.unique(None, "a b");
        assert_eq!(a, "a_b");
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert!(b.len() <= 20 && c.len() <= 20);
    }
}
