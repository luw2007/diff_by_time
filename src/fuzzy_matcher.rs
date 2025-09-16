use fuzzy_matcher::FuzzyMatcher;

/// Represents a match result with score and positions
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub score: i64,
    #[allow(dead_code)]
    pub indices: Vec<usize>,
}

/// FZF-style fuzzy matcher
pub struct FzfMatcher {
    matcher: fuzzy_matcher::skim::SkimMatcherV2,
}

impl FzfMatcher {
    /// Create a new fuzzy matcher
    pub fn new() -> Self {
        Self {
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default(),
        }
    }

    /// Perform fuzzy matching and return match score
    pub fn fuzzy_match(&self, pattern: &str, text: &str) -> Option<MatchResult> {
        self.matcher.fuzzy_indices(text, pattern).map(|(score, indices)| {
            MatchResult {
                score,
                indices,
            }
        })
    }

    /// Perform exact match (priority)
    pub fn exact_match(&self, pattern: &str, text: &str) -> Option<MatchResult> {
        if text.contains(pattern) {
            // Calculate match positions
            let start_pos = text.find(pattern)?;
            let indices: Vec<usize> = (start_pos..start_pos + pattern.len()).collect();

            // Exact match has highest score
            let score = 1000 + (pattern.len() * 10) as i64;

            Some(MatchResult { score, indices })
        } else {
            None
        }
    }

    /// Perform prefix matching
    pub fn prefix_match(&self, pattern: &str, text: &str) -> Option<MatchResult> {
        if text.starts_with(pattern) {
            let indices: Vec<usize> = (0..pattern.len()).collect();
            let score = 800 + (pattern.len() * 8) as i64;
            Some(MatchResult { score, indices })
        } else {
            None
        }
    }

    /// Perform number matching (for serial number filtering)
    pub fn number_match(&self, pattern: &str, text: &str) -> Option<MatchResult> {
        // Check if it's a pure number
        if pattern.chars().all(|c| c.is_ascii_digit()) {
            // Find numbers in text
            if let Some(pos) = text.find(pattern) {
                let indices: Vec<usize> = (pos..pos + pattern.len()).collect();
                let score = 1200; // Number match has very high score
                Some(MatchResult { score, indices })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Comprehensive matching (try different strategies by priority)
    pub fn comprehensive_match(&self, pattern: &str, text: &str) -> Option<MatchResult> {
        if pattern.is_empty() {
            // Empty pattern matches all content
            return Some(MatchResult {
                score: 0,
                indices: Vec::new(),
            });
        }

        // Try different strategies by priority
        if let Some(result) = self.number_match(pattern, text) {
            return Some(result);
        }

        if let Some(result) = self.exact_match(pattern, text) {
            return Some(result);
        }

        if let Some(result) = self.prefix_match(pattern, text) {
            return Some(result);
        }

        // Finally try fuzzy matching
        self.fuzzy_match(pattern, text)
    }

    /// Match and sort multiple items
    pub fn match_and_sort<T>(&self, pattern: &str, items: Vec<(T, String)>) -> Vec<(T, String, MatchResult)> {
        let mut results: Vec<(T, String, MatchResult)> = Vec::new();

        for (item, text) in items {
            if let Some(match_result) = self.comprehensive_match(pattern, &text) {
                results.push((item, text, match_result));
            }
        }

        // Sort by score (higher scores first)
        results.sort_by(|a, b| {
            // First sort by score
            if a.2.score != b.2.score {
                return b.2.score.cmp(&a.2.score);
            }
            // For same scores, sort by text length (shorter first)
            a.1.len().cmp(&b.1.len())
        });

        results
    }

    /// Highlight matched text
    #[allow(dead_code)]
    pub fn highlight_matches(&self, text: &str, indices: &[usize]) -> String {
        if indices.is_empty() {
            return text.to_string();
        }

        let mut result = String::new();
        let mut last_pos = 0;

        for &pos in indices {
            if pos > last_pos {
                result.push_str(&text[last_pos..pos]);
            }
            if pos < text.len() {
                // Use ANSI color codes to highlight matched characters
                result.push_str("\x1b[31m"); // Red color
                result.push(text.chars().nth(pos).unwrap());
                result.push_str("\x1b[0m"); // Reset color
            }
            last_pos = pos + 1;
        }

        if last_pos < text.len() {
            result.push_str(&text[last_pos..]);
        }

        result
    }
}

impl Default for FzfMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_matcher() {
        let matcher = FzfMatcher::new();

        // Test exact match
        let pat = "test";
        let result = matcher.exact_match(pat, "this is a test string");
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 1000 + (pat.len() as i64) * 10);

        // Test prefix match
        let pat = "this";
        let result = matcher.prefix_match(pat, "this is a test");
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 800 + (pat.len() as i64) * 8);

        // Test number match
        let result = matcher.number_match("123", "item 123: test");
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 1200);

        // Test fuzzy match
        let result = matcher.fuzzy_match("tst", "test");
        assert!(result.is_some());

        // Test no match
        let result = matcher.exact_match("xyz", "test string");
        assert!(result.is_none());
    }

    #[test]
    fn test_match_and_sort() {
        let matcher = FzfMatcher::new();
        let items = vec![
            (1, "apple".to_string()),
            (2, "application".to_string()),
            (3, "banana".to_string()),
            (4, "app".to_string()),
        ];

        let results = matcher.match_and_sort("app", items);

        // Should return all items starting with "app", sorted by relevance
        assert_eq!(results.len(), 3);

        // "app" should be at the front (exact match)
        assert_eq!(results[0].0, 4);
        assert_eq!(results[0].1, "app");

        // "apple" and "application" should also have high scores
        assert!(results.iter().any(|(id, _, _)| *id == 1));
        assert!(results.iter().any(|(id, _, _)| *id == 2));
    }

    #[test]
    fn test_highlight_matches() {
        let matcher = FzfMatcher::new();
        let result = matcher.fuzzy_match("tst", "test").unwrap();
        let highlighted = matcher.highlight_matches("test", &result.indices);

        // Should highlight matched characters
        assert!(highlighted.contains("\x1b[31m"));
        assert!(highlighted.contains("\x1b[0m"));
    }
}
