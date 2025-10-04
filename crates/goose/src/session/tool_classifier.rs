//! Tool Classifier - Intelligently extracts operation details from MCP tool calls
//! 
//! This module analyzes tool calls to determine the actual operation being performed,
//! regardless of the MCP extension or tool name used. It works by examining:
//! - Tool name
//! - Tool arguments (command, action, operation, etc.)
//! - File paths and patterns
//! 
//! This enables accurate metrics tracking across different MCP implementations.

use serde_json::Value;

/// Represents a classified tool operation with detailed metadata
#[derive(Debug, Clone)]
pub struct ClassifiedTool {
    /// Original tool name from MCP
    pub original_name: String,
    /// Classified operation category
    pub operation: ToolOperation,
    /// Extension/namespace that provided the tool
    pub extension: Option<String>,
    /// Additional metadata about the operation
    pub metadata: ToolMetadata,
}

/// High-level operation categories
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolOperation {
    FileCreate,
    FileEdit,
    FileRead,
    FileDelete,
    CommandExecute,
    Search,
    Navigate,
    Other(String),
}

/// Additional metadata about the tool operation
#[derive(Debug, Clone, Default)]
pub struct ToolMetadata {
    /// File path if applicable
    pub file_path: Option<String>,
    /// Command being executed if applicable
    pub command: Option<String>,
    /// Search query if applicable
    pub query: Option<String>,
}

impl ClassifiedTool {
    /// Get a standardized name for metrics tracking
    pub fn metrics_name(&self) -> String {
        match &self.operation {
            ToolOperation::FileCreate => "file_create".to_string(),
            ToolOperation::FileEdit => "file_edit".to_string(),
            ToolOperation::FileRead => "file_read".to_string(),
            ToolOperation::FileDelete => "file_delete".to_string(),
            ToolOperation::CommandExecute => "command_execute".to_string(),
            ToolOperation::Search => "search".to_string(),
            ToolOperation::Navigate => "navigate".to_string(),
            ToolOperation::Other(name) => name.clone(),
        }
    }

    /// Get a detailed name including extension prefix
    pub fn detailed_name(&self) -> String {
        if let Some(ext) = &self.extension {
            format!("{}::{}", ext, self.metrics_name())
        } else {
            self.metrics_name()
        }
    }
}

/// Classify a tool call based on its name and arguments
pub fn classify_tool(tool_name: &str, arguments: &Option<serde_json::Map<String, Value>>) -> ClassifiedTool {
    // Extract extension name if present (format: "extension__tool" or "extension::tool")
    let (extension, base_name) = extract_extension_and_name(tool_name);
    
    // Try to classify based on tool name patterns
    let operation = classify_by_name(&base_name);
    
    // If we couldn't classify by name, try arguments
    let (final_operation, metadata) = if matches!(operation, ToolOperation::Other(_)) {
        classify_by_arguments(&base_name, arguments)
    } else {
        (operation, extract_metadata(&base_name, arguments))
    };
    
    ClassifiedTool {
        original_name: tool_name.to_string(),
        operation: final_operation,
        extension,
        metadata,
    }
}

/// Extract extension name and base tool name from a prefixed tool name
fn extract_extension_and_name(tool_name: &str) -> (Option<String>, String) {
    // Try double underscore separator (developer__create_file)
    if let Some((ext, name)) = tool_name.split_once("__") {
        return (Some(ext.to_string()), name.to_string());
    }
    
    // Try double colon separator (developer::create_file)
    if let Some((ext, name)) = tool_name.split_once("::") {
        return (Some(ext.to_string()), name.to_string());
    }
    
    // No extension prefix
    (None, tool_name.to_string())
}

/// Classify operation based on tool name patterns
fn classify_by_name(name: &str) -> ToolOperation {
    let name_lower = name.to_lowercase();
    
    // File operations
    if name_lower.contains("create") && (name_lower.contains("file") || name_lower.contains("write")) {
        return ToolOperation::FileCreate;
    }
    if name_lower.contains("edit") || name_lower.contains("modify") || name_lower.contains("update") 
        || name_lower.contains("replace") || name_lower.contains("patch") {
        return ToolOperation::FileEdit;
    }
    if name_lower.contains("read") || name_lower.contains("view") || name_lower.contains("cat") 
        || name_lower.contains("show") {
        return ToolOperation::FileRead;
    }
    if name_lower.contains("delete") || name_lower.contains("remove") || name_lower.contains("rm") {
        return ToolOperation::FileDelete;
    }
    
    // Command execution
    if name_lower.contains("execute") || name_lower.contains("run") || name_lower.contains("command")
        || name_lower.contains("shell") || name_lower.contains("bash") {
        return ToolOperation::CommandExecute;
    }
    
    // Search operations
    if name_lower.contains("search") || name_lower.contains("find") || name_lower.contains("query") {
        return ToolOperation::Search;
    }
    
    // Navigation
    if name_lower.contains("navigate") || name_lower.contains("goto") || name_lower.contains("open") {
        return ToolOperation::Navigate;
    }
    
    // Couldn't classify by name
    ToolOperation::Other(name.to_string())
}

/// Classify operation based on tool arguments
fn classify_by_arguments(
    tool_name: &str,
    arguments: &Option<serde_json::Map<String, Value>>,
) -> (ToolOperation, ToolMetadata) {
    let mut metadata = ToolMetadata::default();
    
    let args = match arguments {
        Some(args) => args,
        None => return (ToolOperation::Other(tool_name.to_string()), metadata),
    };
    
    // Check for common argument patterns
    
    // text_editor tool with command argument
    if tool_name == "text_editor" || tool_name.contains("editor") {
        if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
            metadata.file_path = args.get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            return match command {
                "write" | "create" => (ToolOperation::FileCreate, metadata),
                "edit" | "replace" | "str_replace" => (ToolOperation::FileEdit, metadata),
                "view" | "read" => (ToolOperation::FileRead, metadata),
                _ => (ToolOperation::Other(format!("text_editor_{}", command)), metadata),
            };
        }
    }
    
    // shell/bash tool
    if tool_name == "shell" || tool_name == "bash" || tool_name.contains("command") {
        metadata.command = args.get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        return (ToolOperation::CommandExecute, metadata);
    }
    
    // Generic file operations based on arguments
    if let Some(path) = args.get("path").or_else(|| args.get("file")).or_else(|| args.get("filename")) {
        metadata.file_path = path.as_str().map(|s| s.to_string());
        
        // Check for operation type in arguments
        if let Some(op) = args.get("operation").or_else(|| args.get("action")).and_then(|v| v.as_str()) {
            return match op {
                "create" | "write" => (ToolOperation::FileCreate, metadata),
                "edit" | "modify" | "update" => (ToolOperation::FileEdit, metadata),
                "read" | "view" => (ToolOperation::FileRead, metadata),
                "delete" | "remove" => (ToolOperation::FileDelete, metadata),
                _ => (ToolOperation::Other(format!("file_{}", op)), metadata),
            };
        }
        
        // Check for content argument (usually means write/create)
        if args.contains_key("content") || args.contains_key("text") {
            return (ToolOperation::FileCreate, metadata);
        }
    }
    
    // Search operations
    if let Some(query) = args.get("query").or_else(|| args.get("search")).and_then(|v| v.as_str()) {
        metadata.query = Some(query.to_string());
        return (ToolOperation::Search, metadata);
    }
    
    // Couldn't classify
    (ToolOperation::Other(tool_name.to_string()), metadata)
}

/// Extract metadata from tool arguments
fn extract_metadata(
    _tool_name: &str,
    arguments: &Option<serde_json::Map<String, Value>>,
) -> ToolMetadata {
    let mut metadata = ToolMetadata::default();
    
    if let Some(args) = arguments {
        // Extract file path
        metadata.file_path = args.get("path")
            .or_else(|| args.get("file"))
            .or_else(|| args.get("filename"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        // Extract command
        metadata.command = args.get("command")
            .or_else(|| args.get("cmd"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        // Extract query
        metadata.query = args.get("query")
            .or_else(|| args.get("search"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_classify_prefixed_tools() {
        let classified = classify_tool("developer__create_file", &None);
        assert_eq!(classified.extension, Some("developer".to_string()));
        assert_eq!(classified.operation, ToolOperation::FileCreate);
        assert_eq!(classified.metrics_name(), "file_create");
    }

    #[test]
    fn test_classify_text_editor_write() {
        let args = json!({
            "command": "write",
            "path": "/test/file.txt",
            "content": "hello"
        });
        let args_map = args.as_object().unwrap().clone();
        
        let classified = classify_tool("text_editor", &Some(args_map));
        assert_eq!(classified.operation, ToolOperation::FileCreate);
        assert_eq!(classified.metadata.file_path, Some("/test/file.txt".to_string()));
    }

    #[test]
    fn test_classify_text_editor_edit() {
        let args = json!({
            "command": "str_replace",
            "path": "/test/file.txt"
        });
        let args_map = args.as_object().unwrap().clone();
        
        let classified = classify_tool("text_editor", &Some(args_map));
        assert_eq!(classified.operation, ToolOperation::FileEdit);
    }

    #[test]
    fn test_classify_shell_command() {
        let args = json!({
            "command": "ls -la"
        });
        let args_map = args.as_object().unwrap().clone();
        
        let classified = classify_tool("shell", &Some(args_map));
        assert_eq!(classified.operation, ToolOperation::CommandExecute);
        assert_eq!(classified.metadata.command, Some("ls -la".to_string()));
    }

    #[test]
    fn test_classify_search() {
        let args = json!({
            "query": "test search"
        });
        let args_map = args.as_object().unwrap().clone();
        
        let classified = classify_tool("llm_search", &Some(args_map));
        assert_eq!(classified.operation, ToolOperation::Search);
    }
}
