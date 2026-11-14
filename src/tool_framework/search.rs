//! The ranking of [`ToolRegistry::search_deferred`](super::ToolRegistry::search_deferred): word
//! overlap between the query and a tool's name and description.

/// The lowercase words of `text`: runs of letters and digits, with `camelCase` split.
pub(super) fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut previous_lower = false;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && previous_lower && !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            previous_lower = c.is_lowercase() || c.is_ascii_digit();
            word.extend(c.to_lowercase());
        } else {
            previous_lower = false;
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// Two points per query word in the name, one per query word in the description.
pub(super) fn score(query: &[String], name: &[String], description: &[String]) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    query
        .iter()
        .filter(|word| seen.insert(word.as_str()))
        .map(|word| {
            let in_name = name.iter().any(|candidate| matches(word, candidate));
            let in_description = description.iter().any(|candidate| matches(word, candidate));
            usize::from(in_name) * 2 + usize::from(in_description)
        })
        .sum()
}

/// Equal, or a plural of the other (`file` and `files`).
fn matches(query: &str, candidate: &str) -> bool {
    query == candidate
        || query.strip_suffix('s') == Some(candidate)
        || candidate.strip_suffix('s') == Some(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_words() {
        assert_eq!(
            words("get_weatherForecast now!"),
            ["get", "weather", "forecast", "now"]
        );
        assert_eq!(words("HTTPServer v2"), ["httpserver", "v2"]);
    }

    #[test]
    fn scores_name_over_description() {
        let query = words("weather files");
        assert_eq!(
            score(&query, &words("get_weather"), &words("Read a file")),
            3
        );
        assert_eq!(score(&query, &words("calc"), &words("Add numbers")), 0);
    }
}
