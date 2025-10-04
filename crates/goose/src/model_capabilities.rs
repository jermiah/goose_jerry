//! Model Capability Checker
//! 
//! This module provides functionality to validate that a model supports required capabilities
//! like tool calling and code generation before starting a session.

use std::collections::HashMap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// Capabilities that a model might support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelCapability {
    /// Model supports function/tool calling
    ToolCalling,
    /// Model is good at code generation
    CodeGeneration,
    /// Model supports streaming responses
    Streaming,
    /// Model supports vision/image inputs
    Vision,
    /// Model supports embeddings
    Embeddings,
}

/// Result of a capability check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityStatus {
    /// Capability is fully supported
    Supported,
    /// Capability is not supported
    NotSupported,
    /// Capability status is unknown (model not in database)
    Unknown,
}

/// Information about a model's capabilities
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub model_name: String,
    pub tool_calling: CapabilityStatus,
    pub code_generation: CapabilityStatus,
    pub streaming: CapabilityStatus,
    pub vision: CapabilityStatus,
    pub embeddings: CapabilityStatus,
}

impl ModelCapabilities {
    /// Check if the model supports a specific capability
    pub fn supports(&self, capability: ModelCapability) -> CapabilityStatus {
        match capability {
            ModelCapability::ToolCalling => self.tool_calling.clone(),
            ModelCapability::CodeGeneration => self.code_generation.clone(),
            ModelCapability::Streaming => self.streaming.clone(),
            ModelCapability::Vision => self.vision.clone(),
            ModelCapability::Embeddings => self.embeddings.clone(),
        }
    }

    /// Check if model has all required capabilities for Goose
    pub fn is_goose_compatible(&self) -> bool {
        matches!(self.tool_calling, CapabilityStatus::Supported | CapabilityStatus::Unknown)
            && matches!(self.code_generation, CapabilityStatus::Supported | CapabilityStatus::Unknown)
    }

    /// Get a human-readable compatibility message
    pub fn get_compatibility_message(&self) -> String {
        let mut issues = Vec::new();

        if self.tool_calling == CapabilityStatus::NotSupported {
            issues.push("❌ Tool calling: Not supported");
        }
        if self.code_generation == CapabilityStatus::NotSupported {
            issues.push("❌ Code generation: Not supported");
        }

        if issues.is_empty() {
            "✅ Model is compatible with Goose".to_string()
        } else {
            format!(
                "⚠️  Model may have compatibility issues:\n{}",
                issues.join("\n")
            )
        }
    }
}

/// Database of known model capabilities
/// This is a curated list based on model documentation and testing
static MODEL_CAPABILITY_DB: Lazy<HashMap<&'static str, (bool, bool)>> = Lazy::new(|| {
    let mut db = HashMap::new();
    
    // Format: (supports_tool_calling, supports_code_generation)
    
    // OpenAI Models - All support tool calling and code generation
    db.insert("gpt-4o", (true, true));
    db.insert("gpt-4o-mini", (true, true));
    db.insert("gpt-4-turbo", (true, true));
    db.insert("gpt-4", (true, true));
    db.insert("gpt-3.5-turbo", (true, true));
    db.insert("o1", (false, true)); // o1 doesn't support tool calling
    db.insert("o1-mini", (false, true));
    db.insert("o1-preview", (false, true));
    db.insert("o3", (false, true));
    db.insert("o3-mini", (false, true));
    db.insert("o4-mini", (false, true));
    
    // Anthropic Models - All support tool calling and code generation
    db.insert("claude-3-opus", (true, true));
    db.insert("claude-3-sonnet", (true, true));
    db.insert("claude-3-haiku", (true, true));
    db.insert("claude-3-5-sonnet", (true, true));
    db.insert("claude-3-5-haiku", (true, true));
    db.insert("claude-sonnet-4", (true, true));
    db.insert("claude-opus-4", (true, true));
    
    // Google Models
    db.insert("gemini-1.5-pro", (true, true));
    db.insert("gemini-1.5-flash", (true, true));
    db.insert("gemini-2.0-flash", (true, true));
    db.insert("gemini-2.5-pro", (true, true));
    db.insert("gemini-exp", (true, true));
    
    // Meta Llama Models - Most support tool calling
    db.insert("llama-3.3-70b", (true, true));
    db.insert("llama-3.2-90b", (true, true));
    db.insert("llama-3.2-11b", (true, true));
    db.insert("llama-3.2-3b", (true, true));
    db.insert("llama-3.2-1b", (true, true));
    db.insert("llama-3.1-405b", (true, true));
    db.insert("llama-3.1-70b", (true, true));
    db.insert("llama-3.1-8b", (true, true));
    db.insert("llama-3-70b", (true, true));
    db.insert("llama-3-8b", (true, true));
    db.insert("llama-2", (false, true)); // Llama 2 doesn't support tool calling
    
    // Mistral Models
    db.insert("mistral-large", (true, true));
    db.insert("mistral-medium", (true, true));
    db.insert("mistral-small", (true, true));
    db.insert("mistral-31-24b", (true, true));
    db.insert("mixtral-8x7b", (true, true));
    db.insert("mixtral-8x22b", (true, true));
    
    // DeepSeek Models
    db.insert("deepseek-chat", (true, true));
    db.insert("deepseek-coder", (true, true));
    db.insert("deepseek-r1", (true, true));
    db.insert("deepseek-v3", (true, true));
    
    // Qwen Models
    db.insert("qwen-2.5-coder", (true, true));
    db.insert("qwen-2.5-72b", (true, true));
    db.insert("qwen-2-72b", (true, true));
    db.insert("qwen3-coder", (true, true));
    
    // Grok Models
    db.insert("grok-2", (true, true));
    db.insert("grok-beta", (true, true));
    db.insert("grok-4", (true, true));
    
    // Cohere Models
    db.insert("command-r-plus", (true, true));
    db.insert("command-r", (true, true));
    db.insert("command", (true, true));
    
    // Models known to NOT support tool calling
    db.insert("gpt-3.5", (false, true));
    db.insert("text-davinci", (false, true));
    db.insert("text-curie", (false, false));
    db.insert("text-babbage", (false, false));
    db.insert("text-ada", (false, false));
    
    db
});

/// Check if a model name matches a pattern in the database
fn find_model_capabilities(model_name: &str) -> Option<(bool, bool)> {
    let model_lower = model_name.to_lowercase();
    
    // First try exact match
    if let Some(&caps) = MODEL_CAPABILITY_DB.get(model_name) {
        return Some(caps);
    }
    
    // Then try pattern matching (e.g., "gpt-4o-2024-05-13" matches "gpt-4o")
    for (pattern, &caps) in MODEL_CAPABILITY_DB.iter() {
        if model_lower.contains(&pattern.to_lowercase()) {
            return Some(caps);
        }
    }
    
    None
}

/// Get capabilities for a specific model
pub fn get_model_capabilities(model_name: &str) -> ModelCapabilities {
    let (tool_calling, code_generation) = find_model_capabilities(model_name)
        .unwrap_or((false, false)); // Default to unknown if not in database
    
    let (tool_status, code_status) = if find_model_capabilities(model_name).is_some() {
        // Model is in database, use definitive status
        (
            if tool_calling { CapabilityStatus::Supported } else { CapabilityStatus::NotSupported },
            if code_generation { CapabilityStatus::Supported } else { CapabilityStatus::NotSupported },
        )
    } else {
        // Model not in database, mark as unknown
        (CapabilityStatus::Unknown, CapabilityStatus::Unknown)
    };
    
    ModelCapabilities {
        model_name: model_name.to_string(),
        tool_calling: tool_status,
        code_generation: code_status,
        streaming: CapabilityStatus::Unknown, // Would need provider-specific check
        vision: CapabilityStatus::Unknown,
        embeddings: CapabilityStatus::Unknown,
    }
}

/// Validation result with detailed information
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_compatible: bool,
    pub capabilities: ModelCapabilities,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

impl ValidationResult {
    /// Check if the user should be warned before proceeding
    pub fn should_warn(&self) -> bool {
        !self.is_compatible || !self.warnings.is_empty()
    }

    /// Get a formatted message for display
    pub fn get_display_message(&self) -> String {
        let mut message = String::new();
        
        message.push_str(&format!("Model: {}\n", self.capabilities.model_name));
        message.push_str(&format!("{}\n", self.capabilities.get_compatibility_message()));
        
        if !self.warnings.is_empty() {
            message.push_str("\nWarnings:\n");
            for warning in &self.warnings {
                message.push_str(&format!("  • {}\n", warning));
            }
        }
        
        if !self.recommendations.is_empty() {
            message.push_str("\nRecommendations:\n");
            for rec in &self.recommendations {
                message.push_str(&format!("  • {}\n", rec));
            }
        }
        
        message
    }
}

/// Validate a model for use with Goose
pub fn validate_model(model_name: &str, provider_name: &str) -> ValidationResult {
    let capabilities = get_model_capabilities(model_name);
    let mut warnings = Vec::new();
    let mut recommendations = Vec::new();
    
    // Check tool calling capability
    match capabilities.tool_calling {
        CapabilityStatus::NotSupported => {
            warnings.push(format!(
                "Model '{}' does not support tool calling, which is required for Goose to function properly.",
                model_name
            ));
            recommendations.push(get_recommended_models(provider_name));
        }
        CapabilityStatus::Unknown => {
            warnings.push(format!(
                "Model '{}' is not in our capability database. Tool calling support is unknown.",
                model_name
            ));
            warnings.push("If the model doesn't support tool calling, Goose will not work correctly.".to_string());
        }
        CapabilityStatus::Supported => {}
    }
    
    // Check code generation capability
    match capabilities.code_generation {
        CapabilityStatus::NotSupported => {
            warnings.push(format!(
                "Model '{}' may not be optimized for code generation.",
                model_name
            ));
        }
        CapabilityStatus::Unknown => {
            // Don't warn for unknown code generation - it's less critical
        }
        CapabilityStatus::Supported => {}
    }
    
    let is_compatible = capabilities.is_goose_compatible();
    
    ValidationResult {
        is_compatible,
        capabilities,
        warnings,
        recommendations,
    }
}

/// Get recommended models for a provider
fn get_recommended_models(provider_name: &str) -> String {
    match provider_name.to_lowercase().as_str() {
        "openai" => "Try: gpt-4o, gpt-4o-mini, or gpt-4-turbo".to_string(),
        "anthropic" => "Try: claude-sonnet-4-20250514, claude-3-5-sonnet-20241022, or claude-3-5-haiku-20241022".to_string(),
        "google" | "gemini" => "Try: gemini-2.5-pro, gemini-2.0-flash-exp, or gemini-1.5-pro".to_string(),
        "groq" => "Try: llama-3.3-70b-versatile or mixtral-8x7b-32768".to_string(),
        "venice" => "Try: llama-3.3-70b or mistral-31-24b".to_string(),
        "openrouter" => "Try: anthropic/claude-sonnet-4, openai/gpt-4o, or google/gemini-2.5-pro".to_string(),
        _ => "Use a model that supports tool/function calling (e.g., gpt-4o, claude-sonnet-4, gemini-2.5-pro)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_compatible_models() {
        // Test OpenAI models
        let caps = get_model_capabilities("gpt-4o");
        assert_eq!(caps.tool_calling, CapabilityStatus::Supported);
        assert_eq!(caps.code_generation, CapabilityStatus::Supported);
        assert!(caps.is_goose_compatible());

        // Test Anthropic models
        let caps = get_model_capabilities("claude-sonnet-4-20250514");
        assert_eq!(caps.tool_calling, CapabilityStatus::Supported);
        assert!(caps.is_goose_compatible());

        // Test Google models
        let caps = get_model_capabilities("gemini-2.5-pro");
        assert_eq!(caps.tool_calling, CapabilityStatus::Supported);
        assert!(caps.is_goose_compatible());
    }

    #[test]
    fn test_known_incompatible_models() {
        // Test o1 models (no tool calling)
        let caps = get_model_capabilities("o1");
        assert_eq!(caps.tool_calling, CapabilityStatus::NotSupported);
        assert!(!caps.is_goose_compatible());

        // Test old GPT models
        let caps = get_model_capabilities("text-davinci-003");
        assert_eq!(caps.tool_calling, CapabilityStatus::NotSupported);
        assert!(!caps.is_goose_compatible());
    }

    #[test]
    fn test_unknown_models() {
        let caps = get_model_capabilities("unknown-model-xyz");
        assert_eq!(caps.tool_calling, CapabilityStatus::Unknown);
        assert_eq!(caps.code_generation, CapabilityStatus::Unknown);
        // Unknown models are considered potentially compatible (benefit of doubt)
        assert!(caps.is_goose_compatible());
    }

    #[test]
    fn test_pattern_matching() {
        // Test that versioned models match base patterns
        let caps = get_model_capabilities("gpt-4o-2024-05-13");
        assert_eq!(caps.tool_calling, CapabilityStatus::Supported);

        let caps = get_model_capabilities("claude-3-5-sonnet-20241022");
        assert_eq!(caps.tool_calling, CapabilityStatus::Supported);
    }

    #[test]
    fn test_validation_result() {
        let result = validate_model("gpt-4o", "openai");
        assert!(result.is_compatible);
        assert!(result.warnings.is_empty());

        let result = validate_model("o1", "openai");
        assert!(!result.is_compatible);
        assert!(!result.warnings.is_empty());
        assert!(!result.recommendations.is_empty());
    }

    #[test]
    fn test_display_messages() {
        let caps = get_model_capabilities("gpt-4o");
        let msg = caps.get_compatibility_message();
        assert!(msg.contains("compatible"));

        let caps = get_model_capabilities("o1");
        let msg = caps.get_compatibility_message();
        assert!(msg.contains("issues"));
    }
}
