use worker::database::PocketBaseClient;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing PocketBase integration...");

    // Initialize client
    let mut client = PocketBaseClient::new();

    // Test connection
    println!("1. Testing connection...");
    match client.test_connection().await {
        Ok(true) => println!("   ✅ Connection successful"),
        Ok(false) => {
            println!("   ❌ Connection failed");
            return Ok(());
        }
        Err(e) => {
            println!("   ❌ Connection error: {}", e);
            return Ok(());
        }
    }

    // Test authentication
    println!("2. Testing authentication...");
    match client.authenticate("admin@pocketbase.local", "123456789").await {
        Ok(_) => println!("   ✅ Authentication successful"),
        Err(e) => {
            println!("   ⚠️ Authentication failed: {} (continuing anyway)", e);
        }
    }

    // Test creating a test game
    println!("3. Testing game creation...");
    let test_game = worker::database::GameRecord {
        id: None,
        name: "Test Game".to_string(),
        max_players: 4,
        status: "waiting".to_string(),
        created: None,
        updated: None,
    };

    match client.save_game(&test_game).await {
        Ok(game_id) => println!("   ✅ Game created with ID: {}", game_id),
        Err(e) => {
            println!("   ❌ Failed to create game: {}", e);
        }
    }

    // Test getting games
    println!("4. Testing game retrieval...");
    match client.get_games().await {
        Ok(games) => {
            println!("   ✅ Retrieved {} games", games.len());
            for game in games.iter().take(3) {
                println!("      - {} ({} players, {})", game.name, game.max_players, game.status);
            }
        }
        Err(e) => {
            println!("   ❌ Failed to get games: {}", e);
        }
    }

    println!("✅ PocketBase integration test completed!");
    Ok(())
}
