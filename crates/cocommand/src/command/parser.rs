use super::types::ParsedCommand;

/// Parse a raw command string, extracting `@app` tags and normalizing the remaining text.
pub fn parse(input: &str) -> ParsedCommand {
    let raw_text = input.to_string();
    let mut tags = Vec::new();
    let mut remaining = Vec::new();

    for token in input.split_whitespace() {
        if is_tag(token) {
            tags.push(token[1..].to_lowercase());
        } else {
            remaining.push(token);
        }
    }

    let normalized_text = remaining.join(" ");

    ParsedCommand {
        raw_text,
        normalized_text,
        tags,
    }
}

/// A valid tag starts with `@` followed by one or more `[a-zA-Z0-9_-]` characters,
/// and the entire token matches (no trailing punctuation).
fn is_tag(token: &str) -> bool {
    if let Some(rest) = token.strip_prefix('@') {
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_and_text() {
        let result = parse("@notes @calendar create");
        assert_eq!(result.tags, vec!["notes", "calendar"]);
        assert_eq!(result.normalized_text, "create");
    }

    #[test]
    fn no_tags() {
        let result = parse("hello world");
        assert_eq!(result.tags, Vec::<String>::new());
        assert_eq!(result.normalized_text, "hello world");
    }

    #[test]
    fn collapses_whitespace() {
        let result = parse("  multiple   spaces  ");
        assert_eq!(result.tags, Vec::<String>::new());
        assert_eq!(result.normalized_text, "multiple spaces");
    }

    #[test]
    fn case_insensitive_tags() {
        let result = parse("@Notes @CALENDAR mixed");
        assert_eq!(result.tags, vec!["notes", "calendar"]);
        assert_eq!(result.normalized_text, "mixed");
    }

    #[test]
    fn preserves_tag_order() {
        let result = parse("@a @b @c order");
        assert_eq!(result.tags, vec!["a", "b", "c"]);
        assert_eq!(result.normalized_text, "order");
    }

    #[test]
    fn empty_input() {
        let result = parse("");
        assert_eq!(result.tags, Vec::<String>::new());
        assert_eq!(result.normalized_text, "");
    }

    #[test]
    fn tag_only_input() {
        let result = parse("@notes");
        assert_eq!(result.tags, vec!["notes"]);
        assert_eq!(result.normalized_text, "");
    }

    #[test]
    fn preserves_raw_text() {
        let input = "  @Notes  hello  world  ";
        let result = parse(input);
        assert_eq!(result.raw_text, input);
    }

    #[test]
    fn at_sign_alone_is_not_tag() {
        let result = parse("@ hello");
        assert_eq!(result.tags, Vec::<String>::new());
        assert_eq!(result.normalized_text, "@ hello");
    }

    #[test]
    fn tag_with_hyphen_and_underscore() {
        let result = parse("@my-app @another_app test");
        assert_eq!(result.tags, vec!["my-app", "another_app"]);
        assert_eq!(result.normalized_text, "test");
    }
}
