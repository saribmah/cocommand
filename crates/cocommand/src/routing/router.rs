use crate::command::ParsedCommand;
use crate::routing::RoutingMetadata;

/// A single routing candidate with score and explanation.
#[derive(Debug, Clone)]
pub struct RouteCandidate {
    pub app_id: String,
    pub score: f64,
    pub explanation: String,
}

/// Result of a routing operation.
#[derive(Debug, Clone)]
pub struct RoutingResult {
    pub candidates: Vec<RouteCandidate>,
}

/// Routes parsed commands to candidate apps based on keyword/verb/object/example matching.
pub struct Router {
    entries: Vec<RoutingMetadata>,
    max_candidates: usize,
}

impl Router {
    /// Create a new router with the default candidate limit (7).
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_candidates: 7,
        }
    }

    /// Create a new router with a custom candidate limit.
    pub fn with_max_candidates(max_candidates: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_candidates,
        }
    }

    /// Register an app's routing metadata. All string fields are lowercased at registration.
    pub fn register(&mut self, metadata: RoutingMetadata) {
        let normalized = RoutingMetadata {
            app_id: metadata.app_id,
            keywords: metadata.keywords.into_iter().map(|s| s.to_lowercase()).collect(),
            examples: metadata.examples.into_iter().map(|s| s.to_lowercase()).collect(),
            verbs: metadata.verbs.into_iter().map(|s| s.to_lowercase()).collect(),
            objects: metadata.objects.into_iter().map(|s| s.to_lowercase()).collect(),
        };
        self.entries.push(normalized);
    }

    /// Route a parsed command to candidate apps.
    ///
    /// Scoring:
    /// - Keyword match: +3 per match
    /// - Verb match: +2 per match
    /// - Object match: +2 per match
    /// - Example substring match: +4 per matching example
    ///
    /// If `ParsedCommand.tags` is non-empty, only apps whose `app_id` is in the
    /// tag set are considered (hard allowlist).
    pub fn route(&self, command: &ParsedCommand) -> RoutingResult {
        let tokens: Vec<String> = command
            .normalized_text
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let input_lower = command.normalized_text.to_lowercase();

        let has_tags = !command.tags.is_empty();

        let mut candidates: Vec<RouteCandidate> = self
            .entries
            .iter()
            .filter(|entry| {
                if has_tags {
                    command.tags.contains(&entry.app_id)
                } else {
                    true
                }
            })
            .filter_map(|entry| {
                let mut score: f64 = 0.0;
                let mut explanations: Vec<String> = Vec::new();

                // Keyword matches (+3 each)
                let matched_keywords: Vec<&String> = entry
                    .keywords
                    .iter()
                    .filter(|kw| tokens.contains(kw))
                    .collect();
                if !matched_keywords.is_empty() {
                    score += matched_keywords.len() as f64 * 3.0;
                    let kw_list: Vec<&str> =
                        matched_keywords.iter().map(|s| s.as_str()).collect();
                    explanations.push(format!("matched keywords: [{}]", kw_list.join(", ")));
                }

                // Verb matches (+2 each)
                let matched_verbs: Vec<&String> = entry
                    .verbs
                    .iter()
                    .filter(|v| tokens.contains(v))
                    .collect();
                if !matched_verbs.is_empty() {
                    score += matched_verbs.len() as f64 * 2.0;
                    let v_list: Vec<&str> =
                        matched_verbs.iter().map(|s| s.as_str()).collect();
                    explanations.push(format!("matched verbs: [{}]", v_list.join(", ")));
                }

                // Object matches (+2 each)
                let matched_objects: Vec<&String> = entry
                    .objects
                    .iter()
                    .filter(|o| tokens.contains(o))
                    .collect();
                if !matched_objects.is_empty() {
                    score += matched_objects.len() as f64 * 2.0;
                    let o_list: Vec<&str> =
                        matched_objects.iter().map(|s| s.as_str()).collect();
                    explanations.push(format!("matched objects: [{}]", o_list.join(", ")));
                }

                // Example substring matches (+4 each)
                let matched_examples: Vec<&String> = entry
                    .examples
                    .iter()
                    .filter(|ex| input_lower.contains(ex.as_str()))
                    .collect();
                if !matched_examples.is_empty() {
                    score += matched_examples.len() as f64 * 4.0;
                    let ex_list: Vec<&str> =
                        matched_examples.iter().map(|s| s.as_str()).collect();
                    explanations.push(format!("matched examples: [{}]", ex_list.join(", ")));
                }

                if score > 0.0 {
                    Some(RouteCandidate {
                        app_id: entry.app_id.clone(),
                        score,
                        explanation: explanations.join("; "),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending, tie-break by app_id ascending
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.app_id.cmp(&b.app_id))
        });

        candidates.truncate(self.max_candidates);

        RoutingResult { candidates }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_command(text: &str, tags: Vec<&str>) -> ParsedCommand {
        ParsedCommand {
            raw_text: text.to_string(),
            normalized_text: text.to_string(),
            tags: tags.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    fn clipboard_app() -> RoutingMetadata {
        RoutingMetadata {
            app_id: "clipboard".to_string(),
            keywords: vec!["copy".into(), "paste".into(), "clipboard".into()],
            examples: vec!["copy this text".into(), "paste from clipboard".into()],
            verbs: vec!["copy".into(), "paste".into(), "cut".into()],
            objects: vec!["clipboard".into(), "text".into(), "selection".into()],
        }
    }

    fn notes_app() -> RoutingMetadata {
        RoutingMetadata {
            app_id: "notes".to_string(),
            keywords: vec!["note".into(), "memo".into(), "write".into()],
            examples: vec!["write a note".into(), "save this memo".into()],
            verbs: vec!["write".into(), "save".into(), "create".into()],
            objects: vec!["note".into(), "memo".into(), "document".into()],
        }
    }

    fn calendar_app() -> RoutingMetadata {
        RoutingMetadata {
            app_id: "calendar".to_string(),
            keywords: vec!["schedule".into(), "meeting".into(), "event".into()],
            examples: vec!["schedule a meeting".into(), "create an event".into()],
            verbs: vec!["schedule".into(), "create".into(), "cancel".into()],
            objects: vec!["meeting".into(), "event".into(), "appointment".into()],
        }
    }

    #[test]
    fn keyword_matching_selects_correct_app() {
        let mut router = Router::new();
        router.register(clipboard_app());
        router.register(notes_app());

        let cmd = make_command("copy this text", vec![]);
        let result = router.route(&cmd);

        assert!(!result.candidates.is_empty());
        assert_eq!(result.candidates[0].app_id, "clipboard");
    }

    #[test]
    fn tag_constraint_returns_only_tagged_apps() {
        let mut router = Router::new();
        router.register(clipboard_app());
        router.register(notes_app());
        router.register(calendar_app());

        // "create" matches both notes (verb) and calendar (verb),
        // but tag restricts to calendar only.
        let cmd = make_command("create an event", vec!["calendar"]);
        let result = router.route(&cmd);

        assert!(!result.candidates.is_empty());
        for candidate in &result.candidates {
            assert_eq!(candidate.app_id, "calendar");
        }
    }

    #[test]
    fn no_tags_returns_all_eligible() {
        let mut router = Router::new();
        router.register(clipboard_app());
        router.register(notes_app());
        router.register(calendar_app());

        // "create" matches notes (verb) and calendar (verb+keyword via "create an event" example)
        let cmd = make_command("create something", vec![]);
        let result = router.route(&cmd);

        let ids: Vec<&str> = result.candidates.iter().map(|c| c.app_id.as_str()).collect();
        // Both notes and calendar have "create" as a verb
        assert!(ids.contains(&"notes"));
        assert!(ids.contains(&"calendar"));
    }

    #[test]
    fn bounded_output_respects_max_candidates() {
        let mut router = Router::with_max_candidates(3);

        // Register 5 apps that all match "open"
        for i in 0..5 {
            router.register(RoutingMetadata {
                app_id: format!("app_{}", i),
                keywords: vec!["open".into()],
                examples: vec![],
                verbs: vec![],
                objects: vec![],
            });
        }

        let cmd = make_command("open", vec![]);
        let result = router.route(&cmd);

        assert_eq!(result.candidates.len(), 3);
    }

    #[test]
    fn explanation_strings_are_non_empty() {
        let mut router = Router::new();
        router.register(clipboard_app());

        let cmd = make_command("copy text from clipboard", vec![]);
        let result = router.route(&cmd);

        assert!(!result.candidates.is_empty());
        for candidate in &result.candidates {
            assert!(!candidate.explanation.is_empty());
        }
    }

    #[test]
    fn deterministic_ordering() {
        let mut router = Router::new();
        router.register(clipboard_app());
        router.register(notes_app());
        router.register(calendar_app());

        let cmd = make_command("copy this text", vec![]);
        let result1 = router.route(&cmd);
        let result2 = router.route(&cmd);

        assert_eq!(result1.candidates.len(), result2.candidates.len());
        for (a, b) in result1.candidates.iter().zip(result2.candidates.iter()) {
            assert_eq!(a.app_id, b.app_id);
            assert_eq!(a.score, b.score);
            assert_eq!(a.explanation, b.explanation);
        }
    }

    #[test]
    fn zero_score_apps_not_returned() {
        let mut router = Router::new();
        router.register(clipboard_app());
        router.register(notes_app());

        let cmd = make_command("schedule a meeting", vec![]);
        let result = router.route(&cmd);

        // Neither clipboard nor notes should match "schedule a meeting"
        assert!(result.candidates.is_empty());
    }

    #[test]
    fn empty_registry_returns_empty_candidates() {
        let router = Router::new();

        let cmd = make_command("do something", vec![]);
        let result = router.route(&cmd);

        assert!(result.candidates.is_empty());
    }
}
