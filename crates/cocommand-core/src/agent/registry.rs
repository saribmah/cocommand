//! Agent registry for creating agent configurations.
//!
//! This module provides factory functions for agent configurations,
//! following opencode's pattern of clean separation between:
//! - Agent configuration (this module)
//! - Prompt construction (prompt module)
//! - Context building (context module)

use super::config::AgentConfig;
use super::prompt::base::build_instructions;

/// Create the default agent configuration.
///
/// This is the primary agent used for processing user commands.
/// Uses conservative settings for reliable tool selection.
pub fn default_agent() -> AgentConfig {
    AgentConfig {
        id: "primary".to_string(),
        instructions: build_instructions(),
        temperature: 0.2,
        max_output_tokens: 600,
    }
}

/// Create a verbose agent configuration for debugging.
///
/// Higher token limit for more detailed responses.
pub fn verbose_agent() -> AgentConfig {
    AgentConfig {
        id: "verbose".to_string(),
        instructions: build_instructions(),
        temperature: 0.3,
        max_output_tokens: 1200,
    }
}

/// Create an agent configuration with custom instructions.
pub fn agent_with_instructions(instructions: String) -> AgentConfig {
    AgentConfig {
        id: "custom".to_string(),
        instructions,
        temperature: 0.2,
        max_output_tokens: 600,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_agent() {
        let config = default_agent();
        assert_eq!(config.id, "primary");
        assert!(config.instructions.contains("cocommand"));
        assert!(config.temperature > 0.0 && config.temperature < 1.0);
    }

    #[test]
    fn test_verbose_agent() {
        let config = verbose_agent();
        assert_eq!(config.id, "verbose");
        assert!(config.max_output_tokens > default_agent().max_output_tokens);
    }

    #[test]
    fn test_agent_with_instructions() {
        let config = agent_with_instructions("Custom agent".to_string());
        assert_eq!(config.id, "custom");
        assert_eq!(config.instructions, "Custom agent");
    }
}
