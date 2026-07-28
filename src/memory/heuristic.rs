#![allow(dead_code)]

use std::collections::HashSet;

const MIN_CONTENT_LENGTH: usize = 10;
const MAX_CONTENT_LENGTH: usize = 10000;
const MIN_WORDS: usize = 3;
const DUPLICATE_CHECK_PREFIX_LEN: usize = 100;

const JUNK_PATTERNS: &[&str] = &[
    "test test",
    "lorem ipsum",
    "asdf",
    "qwer",
    "12345",
    "null",
    "undefined",
    "none",
    "todo",
    "fixme",
    "hack",
    "placeholder",
];

#[derive(Debug, Clone)]
pub struct HeuristicResult {
    pub passed: bool,
    pub reason: Option<String>,
}

pub fn check_content(content: &str) -> HeuristicResult {
    let trimmed = content.trim();

    if trimmed.len() < MIN_CONTENT_LENGTH {
        return HeuristicResult {
            passed: false,
            reason: Some(format!("content too short: {} chars (min {})", trimmed.len(), MIN_CONTENT_LENGTH)),
        };
    }

    if trimmed.len() > MAX_CONTENT_LENGTH {
        return HeuristicResult {
            passed: false,
            reason: Some(format!("content too long: {} chars (max {})", trimmed.len(), MAX_CONTENT_LENGTH)),
        };
    }

    let word_count = trimmed.split_whitespace().count();
    if word_count < MIN_WORDS {
        return HeuristicResult {
            passed: false,
            reason: Some(format!("too few words: {} (min {})", word_count, MIN_WORDS)),
        };
    }

    let lower = trimmed.to_lowercase();
    for pattern in JUNK_PATTERNS {
        if lower == *pattern {
            return HeuristicResult {
                passed: false,
                reason: Some(format!("junk content matched pattern: '{}'", pattern)),
            };
        }
    }

    if trimmed.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()) {
        return HeuristicResult {
            passed: false,
            reason: Some("content is only digits".to_string()),
        };
    }

    let unique_chars: HashSet<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    if unique_chars.len() <= 2 && trimmed.len() > 20 {
        return HeuristicResult {
            passed: false,
            reason: Some("content has very low character diversity".to_string()),
        };
    }

    HeuristicResult {
        passed: true,
        reason: None,
    }
}

pub fn find_similar_in_batch(contents: &[&str]) -> Vec<(usize, usize, f64)> {
    let mut duplicates: Vec<(usize, usize, f64)> = Vec::new();

    for i in 0..contents.len() {
        let prefix_i = contents[i][..contents[i].len().min(DUPLICATE_CHECK_PREFIX_LEN)].to_lowercase();
        for j in (i + 1)..contents.len() {
            let prefix_j = contents[j][..contents[j].len().min(DUPLICATE_CHECK_PREFIX_LEN)].to_lowercase();
            if prefix_i == prefix_j {
                let similarity = compute_jaccard(contents[i], contents[j]);
                if similarity > 0.8 {
                    duplicates.push((i, j, similarity));
                }
            }
        }
    }

    duplicates
}

fn compute_jaccard(a: &str, b: &str) -> f64 {
    let words_a: HashSet<&str> = a.split_whitespace().collect();
    let words_b: HashSet<&str> = b.split_whitespace().collect();

    if words_a.is_empty() || words_b.is_empty() {
        return 0.0;
    }

    let intersection: HashSet<&&str> = words_a.intersection(&words_b).collect();
    let union_count = words_a.len() + words_b.len() - intersection.len();

    if union_count == 0 {
        return 0.0;
    }

    intersection.len() as f64 / union_count as f64
}

pub fn validate_layer_for_project(layer: &str, project_id: Option<i64>) -> Result<(), String> {
    match layer {
        "global_profile" => {
            if project_id.is_some() {
                Err("global_profile layer must have project_id = NULL".to_string())
            } else {
                Ok(())
            }
        }
        "project" | "episodic" | "working" => {
            if project_id.is_none() {
                Err(format!("{} layer requires project_id", layer))
            } else {
                Ok(())
            }
        }
        other => Err(format!("invalid layer: {}", other)),
    }
}
