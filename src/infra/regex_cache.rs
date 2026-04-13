use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use regex::Regex;

static REGEX_CACHE: LazyLock<Mutex<HashMap<String, Regex>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_or_compile(pattern: &str) -> Option<Regex> {
    let mut cache = REGEX_CACHE.lock().unwrap();
    if let Some(re) = cache.get(pattern) {
        return Some(re.clone());
    }
    let re = Regex::new(pattern).ok()?;
    cache.insert(pattern.to_string(), re.clone());
    Some(re)
}

/// Match a regex pattern against a text string, returning captures if matched.
/// Returns `Some(captures)` where captures[0] is the full match, captures[1] is the first group, etc.
pub fn regex_match(pattern: &str, text: &str) -> Option<Vec<String>> {
    let re = get_or_compile(pattern)?;
    let caps = re.captures(text)?;
    Some(caps.iter().map(|m| m.unwrap().as_str().to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_match_simple() {
        let result = regex_match("^app-(.+)$", "app-server");
        assert_eq!(result, Some(vec!["app-server".to_string(), "server".to_string()]));
    }

    #[test]
    fn regex_match_no_match() {
        let result = regex_match("^app-(.+)$", "other-crate");
        assert!(result.is_none());
    }

    #[test]
    fn regex_match_middle() {
        let result = regex_match("^app-(.+)-entity$", "app-foo-entity");
        assert_eq!(result, Some(vec!["app-foo-entity".to_string(), "foo".to_string()]));
    }

    #[test]
    fn regex_match_requires_content() {
        let result = regex_match("^app-cli-(.+)$", "app-cli-");
        assert!(result.is_none());
    }

}
