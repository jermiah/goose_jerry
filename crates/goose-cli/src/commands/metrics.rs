use anyhow::Result;
use console::style;
use goose::session::{ensure_session_dir, legacy, SessionManager};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SessionMetrics {
    // Session Info
    session_id: String,
    session_name: String,
    working_dir: String,
    
    // Time Metrics
    created_at: String,
    updated_at: String,
    duration_minutes: f64,
    
    // Token Metrics
    total_tokens: i32,
    input_tokens: i32,
    output_tokens: i32,
    accumulated_total_tokens: i32,
    accumulated_input_tokens: i32,
    accumulated_output_tokens: i32,
    tokens_per_minute: f64,
    
    // Message Metrics
    message_count: usize,
    user_messages: usize,
    assistant_messages: usize,
    messages_per_minute: f64,
    
    // Tool Usage (from message content analysis)
    tool_uses: usize,
    files_created: usize,
    files_modified: usize,
    files_read: usize,
    commands_executed: usize,
    
    // Quality Metrics
    error_count: usize,
    retry_count: usize,
    error_rate: f64,
    success_rate: f64,
    
    // Performance
    files_per_minute: f64,
    
    // Environment
    recipe_name: Option<String>,
    schedule_id: Option<String>,
}

pub async fn handle_metrics(
    session_id: Option<String>,
    detailed: bool,
    export_json: bool,
) -> Result<()> {
    // Get the session to analyze
    let target_session_id = if let Some(id) = session_id {
        // Try to find by ID or name
        let sessions = SessionManager::list_sessions().await?;
        sessions
            .iter()
            .find(|s| s.id == id || s.description == id)
            .map(|s| s.id.clone())
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?
    } else {
        // Get the latest session
        let sessions = SessionManager::list_sessions().await?;
        if sessions.is_empty() {
            anyhow::bail!("No sessions found. Run a goose session first, then try analyzing again.");
        }
        sessions[0].id.clone()
    };
    
    // Try to get full session data with messages from database
    let session = match SessionManager::get_session(&target_session_id, true).await {
        Ok(s) => {
            eprintln!("{}", style("✓ Successfully loaded session from database").green());
            eprintln!();
            s
        },
        Err(db_error) => {
            // Database failed, try loading from JSONL file
            eprintln!("{}", style("⚠ Warning: Cannot load from database").yellow().bold());
            eprintln!("  Error type: {}", std::any::type_name_of_val(&db_error));
            eprintln!("  Error message: {}", db_error);
            eprintln!("  Error debug: {:?}", db_error);
            eprintln!();
            eprintln!("  {} Attempting to load from JSONL file...", style("→").cyan());
            eprintln!();
            
            // Try to load from JSONL file
            let session_dir = ensure_session_dir()?;
            let jsonl_path = session_dir.join(format!("{}.jsonl", target_session_id));
            
            match legacy::load_session(&target_session_id, &jsonl_path) {
                Ok(jsonl_session) => {
                    eprintln!("{}", style("✓ Successfully loaded from JSONL file").green());
                    eprintln!();
                    jsonl_session
                }
                Err(jsonl_error) => {
                    // Both database and JSONL failed, try basic session info
                    eprintln!("{}", style("Warning: Cannot load from JSONL either").yellow());
                    eprintln!("  Error: {}", jsonl_error);
                    eprintln!();
                    
                    match SessionManager::get_session(&target_session_id, false).await {
                        Ok(basic_session) => {
                            eprintln!("{}", style("Showing basic metrics without message analysis:").cyan());
                            eprintln!();
                            display_basic_metrics(&basic_session);
                            return Ok(());
                        }
                        Err(e3) => {
                            eprintln!("{}", style("Error: Cannot load session at all").red().bold());
                            eprintln!("  Database error: {}", db_error);
                            eprintln!("  JSONL error: {}", jsonl_error);
                            eprintln!("  Basic load error: {}", e3);
                            return Err(e3);
                        }
                    }
                }
            }
        }
    };
    
    // Calculate metrics
    let metrics = calculate_metrics(&session).await?;
    
    // Display metrics
    display_metrics(&metrics, detailed);
    
    // Export if requested
    if export_json {
        let json = serde_json::to_string_pretty(&metrics)?;
        println!("\n{}", style("=== JSON EXPORT ===").cyan().bold());
        println!("{}", json);
    }
    
    Ok(())
}

async fn calculate_metrics(session: &goose::session::Session) -> Result<SessionMetrics> {
    use chrono::{DateTime, NaiveDateTime};
    
    // Parse timestamps - handle both RFC3339 and database format
    eprintln!("  → Parsing timestamps...");
    
    // Helper function to parse timestamps in multiple formats
    let parse_timestamp = |ts: &str| -> Result<DateTime<chrono::FixedOffset>> {
        // Try RFC3339 first
        if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
            return Ok(dt);
        }
        
        // Try database format: "2025-09-29 07:00:18"
        if let Ok(naive) = NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S") {
            // Assume UTC timezone
            let utc_dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc);
            return Ok(utc_dt.into());
        }
        
        Err(anyhow::anyhow!("Could not parse timestamp '{}' in any known format", ts))
    };
    
    let created = parse_timestamp(&session.created_at)
        .map_err(|e| anyhow::anyhow!("Failed to parse created_at: {}", e))?;
    let updated = parse_timestamp(&session.updated_at)
        .map_err(|e| anyhow::anyhow!("Failed to parse updated_at: {}", e))?;
    
    let duration = updated.signed_duration_since(created);
    let duration_minutes = duration.num_seconds() as f64 / 60.0;
    
    // Get conversation messages
    eprintln!("  → Checking conversation data...");
    let conversation = session.conversation.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No conversation data available in session"))?;
    
    eprintln!("  → Getting messages from conversation...");
    
    let messages = conversation.messages();
    let message_count = messages.len();
    
    // Count user vs assistant messages
    let user_messages = messages.iter()
        .filter(|m| matches!(m.role, rmcp::model::Role::User))
        .count();
    let assistant_messages = messages.iter()
        .filter(|m| matches!(m.role, rmcp::model::Role::Assistant))
        .count();
    
    // Get tool usage statistics from database
    eprintln!("  → Fetching tool usage data from database...");
    
    let tool_stats = goose::session::SessionManager::get_tool_stats(&session.id)
        .await
        .unwrap_or_else(|_| goose::session::session_manager::ToolStats {
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            cancelled_calls: 0,
            avg_duration_ms: 0.0,
            calls_by_tool: std::collections::HashMap::new(),
            calls_by_operation: std::collections::HashMap::new(),
            file_operations: Vec::new(),
        });
    
    let tool_uses = tool_stats.total_calls;
    let error_count = tool_stats.failed_calls;
    
    // Use enhanced operation_type data from database if available
    // This provides accurate classification from tool_classifier
    let mut files_created = 0;
    let mut files_modified = 0;
    let mut files_read = 0;
    let mut commands_executed = 0;
    
    if !tool_stats.calls_by_operation.is_empty() {
        // Use the accurate operation_type data from the database
        eprintln!("  ✓ Using enhanced operation tracking data");
        
        files_created = *tool_stats.calls_by_operation.get("file_create").unwrap_or(&0);
        files_modified = *tool_stats.calls_by_operation.get("file_edit").unwrap_or(&0);
        files_read = *tool_stats.calls_by_operation.get("file_read").unwrap_or(&0);
        commands_executed = *tool_stats.calls_by_operation.get("command_execute").unwrap_or(&0);
    } else {
        // Fallback to name-based classification for older sessions
        eprintln!("  ⚠ Using fallback name-based classification (legacy session)");
        
        for (tool_name, count) in &tool_stats.calls_by_tool {
            let name_lower = tool_name.to_lowercase();
            
            // File creation tools
            if name_lower.contains("create") || name_lower.contains("write") {
                files_created += count;
            }
            // File modification tools  
            else if name_lower.contains("edit") || name_lower.contains("modify") 
                || name_lower.contains("replace") || name_lower.contains("patch") {
                files_modified += count;
            }
            // File reading tools
            else if name_lower.contains("read") || name_lower.contains("view") 
                || name_lower.contains("cat") || name_lower.contains("show") {
                files_read += count;
            }
            // Command execution tools
            else if name_lower.contains("execute") || name_lower.contains("command")
                || name_lower.contains("shell") || name_lower.contains("bash") 
                || name_lower.contains("run") {
                commands_executed += count;
            }
            // Special case: text_editor tool (needs argument inspection)
            // For now, count it as file modification
            else if tool_name == "text_editor" {
                files_modified += count;
            }
        }
    }
    
    // Analyze messages for retry patterns
    eprintln!("  → Analyzing messages for retry patterns...");
    let mut retry_count = 0;
    for message in messages.iter() {
        let content_str = serde_json::to_string(&message.content)?;
        let content_lower = content_str.to_lowercase();
        if content_lower.contains("retry") || content_lower.contains("try again") {
            retry_count += 1;
        }
    }
    
    if tool_uses > 0 {
        eprintln!("  ✓ Found {} tool calls in database", tool_uses);
    } else {
        eprintln!("  ⚠ No tool usage data found (session may predate tool tracking)");
    }
    
    // Calculate rates
    let messages_per_minute = if duration_minutes > 0.0 {
        message_count as f64 / duration_minutes
    } else {
        0.0
    };
    
    let tokens_per_minute = if duration_minutes > 0.0 {
        session.accumulated_total_tokens.unwrap_or(0) as f64 / duration_minutes
    } else {
        0.0
    };
    
    let files_per_minute = if duration_minutes > 0.0 {
        (files_created + files_modified) as f64 / duration_minutes
    } else {
        0.0
    };
    
    let error_rate = if message_count > 0 {
        (error_count as f64 / message_count as f64) * 100.0
    } else {
        0.0
    };
    
    let success_rate = if tool_uses > 0 {
        ((tool_uses - error_count) as f64 / tool_uses as f64) * 100.0
    } else {
        100.0
    };
    
    // Get recipe title if available
    let recipe_name = session.recipe.as_ref().map(|r| r.title.clone());
    
    Ok(SessionMetrics {
        session_id: session.id.clone(),
        session_name: session.description.clone(),
        working_dir: session.working_dir.to_string_lossy().to_string(),
        created_at: session.created_at.clone(),
        updated_at: session.updated_at.clone(),
        duration_minutes,
        total_tokens: session.total_tokens.unwrap_or(0),
        input_tokens: session.input_tokens.unwrap_or(0),
        output_tokens: session.output_tokens.unwrap_or(0),
        accumulated_total_tokens: session.accumulated_total_tokens.unwrap_or(0),
        accumulated_input_tokens: session.accumulated_input_tokens.unwrap_or(0),
        accumulated_output_tokens: session.accumulated_output_tokens.unwrap_or(0),
        tokens_per_minute,
        message_count,
        user_messages,
        assistant_messages,
        messages_per_minute,
        tool_uses,
        files_created,
        files_modified,
        files_read,
        commands_executed,
        error_count,
        retry_count,
        error_rate,
        success_rate,
        files_per_minute,
        recipe_name,
        schedule_id: session.schedule_id.clone(),
    })
}

fn display_metrics(metrics: &SessionMetrics, detailed: bool) {
    println!();
    println!("{}", style("=== GOOSE SESSION METRICS ===").cyan().bold());
    println!();
    
    // Session Info
    println!("{}", style("SESSION INFO").cyan().bold());
    println!("  Session ID:        {}", metrics.session_id);
    println!("  Session Name:      {}", metrics.session_name);
    println!("  Working Directory: {}", metrics.working_dir);
    if let Some(recipe) = &metrics.recipe_name {
        println!("  Recipe:            {}", recipe);
    }
    if let Some(schedule) = &metrics.schedule_id {
        println!("  Schedule ID:       {}", schedule);
    }
    println!();
    
    // Time Metrics
    println!("{}", style("TIME METRICS").cyan().bold());
    println!("  Created:           {}", metrics.created_at);
    println!("  Updated:           {}", metrics.updated_at);
    println!("  Duration:          {:.2} minutes", metrics.duration_minutes);
    println!();
    
    // Token Metrics
    println!("{}", style("TOKEN METRICS").cyan().bold());
    println!("  Total Tokens:      {}", metrics.accumulated_total_tokens);
    println!("  Input Tokens:      {}", metrics.accumulated_input_tokens);
    println!("  Output Tokens:     {}", metrics.accumulated_output_tokens);
    println!("  Tokens/Minute:     {:.0}", metrics.tokens_per_minute);
    println!();
    
    // Message Metrics
    println!("{}", style("MESSAGE METRICS").cyan().bold());
    println!("  Total Messages:    {}", metrics.message_count);
    println!("  User Messages:     {}", metrics.user_messages);
    println!("  Assistant Messages: {}", metrics.assistant_messages);
    println!("  Messages/Minute:   {:.2}", metrics.messages_per_minute);
    println!();
    
    // Tool Usage
    println!("{}", style("TOOL USAGE").cyan().bold());
    println!("  Total Tool Uses:   {}", metrics.tool_uses);
    println!("  Files Created:     {}", metrics.files_created);
    println!("  Files Modified:    {}", metrics.files_modified);
    println!("  Files Read:        {}", metrics.files_read);
    println!("  Commands Executed: {}", metrics.commands_executed);
    println!("  Files/Minute:      {:.2}", metrics.files_per_minute);
    println!();
    
    // Quality Metrics
    println!("{}", style("QUALITY METRICS").cyan().bold());
    println!("  Errors:            {}", metrics.error_count);
    println!("  Retries:           {}", metrics.retry_count);
    println!("  Error Rate:        {:.2}%", metrics.error_rate);
    println!("  Success Rate:      {:.2}%", metrics.success_rate);
    println!();
    
    
    if detailed {
        display_detailed_metrics(metrics);
    }
}

fn display_basic_metrics(session: &goose::session::Session) {
    use chrono::DateTime;
    
    println!();
    println!("{}", style("=== BASIC SESSION METRICS ===").cyan().bold());
    println!();
    
    println!("{}", style("SESSION INFO").cyan().bold());
    println!("  Session ID:        {}", session.id);
    println!("  Session Name:      {}", session.description);
    println!("  Working Directory: {}", session.working_dir.display());
    if let Some(recipe) = &session.recipe {
        println!("  Recipe:            {}", recipe.title);
    }
    if let Some(schedule) = &session.schedule_id {
        println!("  Schedule ID:       {}", schedule);
    }
    println!();
    
    println!("{}", style("TIME METRICS").cyan().bold());
    println!("  Created:           {}", session.created_at);
    println!("  Updated:           {}", session.updated_at);
    
    if let (Ok(created), Ok(updated)) = (
        DateTime::parse_from_rfc3339(&session.created_at),
        DateTime::parse_from_rfc3339(&session.updated_at)
    ) {
        let duration = updated.signed_duration_since(created);
        let duration_minutes = duration.num_seconds() as f64 / 60.0;
        println!("  Duration:          {:.2} minutes", duration_minutes);
    }
    println!();
    
    println!("{}", style("TOKEN METRICS").cyan().bold());
    println!("  Total Tokens:      {}", session.accumulated_total_tokens.unwrap_or(0));
    println!("  Input Tokens:      {}", session.accumulated_input_tokens.unwrap_or(0));
    println!("  Output Tokens:     {}", session.accumulated_output_tokens.unwrap_or(0));
    println!();
    
    println!("{}", style("MESSAGE COUNT").cyan().bold());
    println!("  Total Messages:    {}", session.message_count);
    println!();
    
    println!("{}", style("NOTE").yellow().bold());
    println!("  Detailed message analysis unavailable due to database format issue.");
    println!("  This is a known limitation with the current session data format.");
    println!();
}

fn display_detailed_metrics(metrics: &SessionMetrics) {
    println!("{}", style("=== DETAILED METRICS ===").yellow().bold());
    println!();
    
    println!("{}", style("Efficiency Breakdown:").yellow());
    println!("  Avg tokens per message:    {:.0}", 
        metrics.accumulated_total_tokens as f64 / metrics.message_count.max(1) as f64);
    println!("  Avg files per tool use:    {:.2}", 
        (metrics.files_created + metrics.files_modified) as f64 / metrics.tool_uses.max(1) as f64);
    println!();
    
    println!("{}", style("Message Distribution:").yellow());
    println!("  User message ratio:        {:.1}%", 
        (metrics.user_messages as f64 / metrics.message_count.max(1) as f64) * 100.0);
    println!("  Assistant message ratio:   {:.1}%", 
        (metrics.assistant_messages as f64 / metrics.message_count.max(1) as f64) * 100.0);
    println!();
    
    println!("{}", style("Tool Usage Patterns:").yellow());
    println!("  Create operations:         {:.1}%", 
        (metrics.files_created as f64 / metrics.tool_uses.max(1) as f64) * 100.0);
    println!("  Modify operations:         {:.1}%", 
        (metrics.files_modified as f64 / metrics.tool_uses.max(1) as f64) * 100.0);
    println!("  Read operations:           {:.1}%", 
        (metrics.files_read as f64 / metrics.tool_uses.max(1) as f64) * 100.0);
    println!("  Command executions:        {:.1}%", 
        (metrics.commands_executed as f64 / metrics.tool_uses.max(1) as f64) * 100.0);
    println!();
}
