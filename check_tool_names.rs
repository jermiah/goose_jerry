// Quick diagnostic to see what tool names are actually in the database
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
            format!("{}\\AppData\\Roaming\\Block\\goose\\data\\sessions\\sessions.db", home)
        });
    
    println!("Checking database: {}\n", db_path);
    
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(false);
    
    let pool = SqlitePool::connect_with(options).await?;
    
    // Get latest session
    let session_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM sessions ORDER BY updated_at DESC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await?;
    
    if let Some(sid) = session_id {
        println!("Latest session: {}\n", sid);
        
        // Get all tool names from tool_events
        let tool_names = sqlx::query_as::<_, (String, i64)>(
            "SELECT tool_name, COUNT(*) as count FROM tool_events WHERE session_id = ? GROUP BY tool_name ORDER BY count DESC"
        )
        .bind(&sid)
        .fetch_all(&pool)
        .await?;
        
        if tool_names.is_empty() {
            println!("No tool events found for this session");
        } else {
            println!("Tool names in database:");
            for (name, count) in tool_names {
                println!("  {} → {} calls", name, count);
            }
        }
    } else {
        println!("No sessions found");
    }
    
    Ok(())
}
