use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, FromRow};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Player {
    id: i64,
    name: String,
    wins: i64,
    losses: i64,
    points: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct NewPlayer {
    name: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Game {
    id: i64,
    team1_player1_id: i64,
    team1_player2_id: i64,
    team2_player1_id: i64,
    team2_player2_id: i64,
    team1_score: i64,
    team2_score: i64,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NewGame {
    team1_player1_id: i64,
    team1_player2_id: i64,
    team2_player1_id: i64,
    team2_player2_id: i64,
    team1_score: i64,
    team2_score: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct LeaderboardEntry {
    id: i64,
    name: String,
    wins: i64,
    losses: i64,
    points: i64,
    win_rate: f64,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rallito=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Setup database
    let db = SqlitePool::connect("sqlite:rallito.db")
        .await
        .expect("Failed to connect to database");

    // Run migrations
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS players (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            wins INTEGER DEFAULT 0,
            losses INTEGER DEFAULT 0,
            points INTEGER DEFAULT 0
        )
        "#,
    )
    .execute(&db)
    .await
    .expect("Failed to create players table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS games (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            team1_player1_id INTEGER NOT NULL,
            team1_player2_id INTEGER NOT NULL,
            team2_player1_id INTEGER NOT NULL,
            team2_player2_id INTEGER NOT NULL,
            team1_score INTEGER NOT NULL,
            team2_score INTEGER NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (team1_player1_id) REFERENCES players(id),
            FOREIGN KEY (team1_player2_id) REFERENCES players(id),
            FOREIGN KEY (team2_player1_id) REFERENCES players(id),
            FOREIGN KEY (team2_player2_id) REFERENCES players(id)
        )
        "#,
    )
    .execute(&db)
    .await
    .expect("Failed to create games table");

    let state = AppState { db };

    // Build router
    let app = Router::new()
        .route("/", get(root))
        .route("/api/players", get(list_players).post(create_player))
        .route("/api/players/:id", get(get_player))
        .route("/api/games", get(list_games).post(create_game))
        .route("/api/leaderboard", get(get_leaderboard))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

async fn list_players(State(state): State<AppState>) -> Result<Json<Vec<Player>>, StatusCode> {
    let players = sqlx::query_as::<_, Player>("SELECT * FROM players ORDER BY name")
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(players))
}

async fn get_player(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Player>, StatusCode> {
    let player = sqlx::query_as::<_, Player>("SELECT * FROM players WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(player))
}

async fn create_player(
    State(state): State<AppState>,
    Json(new_player): Json<NewPlayer>,
) -> Result<Json<Player>, StatusCode> {
    let result = sqlx::query("INSERT INTO players (name) VALUES (?)")
        .bind(&new_player.name)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let player = sqlx::query_as::<_, Player>("SELECT * FROM players WHERE id = ?")
        .bind(result.last_insert_rowid())
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(player))
}

async fn list_games(State(state): State<AppState>) -> Result<Json<Vec<Game>>, StatusCode> {
    let games = sqlx::query_as::<_, Game>("SELECT * FROM games ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(games))
}

async fn create_game(
    State(state): State<AppState>,
    Json(new_game): Json<NewGame>,
) -> Result<Json<Game>, StatusCode> {
    // Insert the game
    let result = sqlx::query(
        r#"
        INSERT INTO games (team1_player1_id, team1_player2_id, team2_player1_id, team2_player2_id, team1_score, team2_score)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(new_game.team1_player1_id)
    .bind(new_game.team1_player2_id)
    .bind(new_game.team2_player1_id)
    .bind(new_game.team2_player2_id)
    .bind(new_game.team1_score)
    .bind(new_game.team2_score)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Determine winner and update player stats
    let (winner_ids, loser_ids) = if new_game.team1_score > new_game.team2_score {
        (
            vec![new_game.team1_player1_id, new_game.team1_player2_id],
            vec![new_game.team2_player1_id, new_game.team2_player2_id],
        )
    } else {
        (
            vec![new_game.team2_player1_id, new_game.team2_player2_id],
            vec![new_game.team1_player1_id, new_game.team1_player2_id],
        )
    };

    // Update winners
    for player_id in winner_ids {
        sqlx::query("UPDATE players SET wins = wins + 1, points = points + 3 WHERE id = ?")
            .bind(player_id)
            .execute(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Update losers
    for player_id in loser_ids {
        sqlx::query("UPDATE players SET losses = losses + 1 WHERE id = ?")
            .bind(player_id)
            .execute(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Fetch and return the created game
    let game = sqlx::query_as::<_, Game>("SELECT * FROM games WHERE id = ?")
        .bind(result.last_insert_rowid())
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(game))
}

async fn get_leaderboard(
    State(state): State<AppState>,
) -> Result<Json<Vec<LeaderboardEntry>>, StatusCode> {
    let players = sqlx::query_as::<_, Player>(
        "SELECT * FROM players ORDER BY points DESC, wins DESC, name ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let leaderboard: Vec<LeaderboardEntry> = players
        .into_iter()
        .map(|p| {
            let total_games = p.wins + p.losses;
            let win_rate = if total_games > 0 {
                (p.wins as f64 / total_games as f64) * 100.0
            } else {
                0.0
            };

            LeaderboardEntry {
                id: p.id,
                name: p.name,
                wins: p.wins,
                losses: p.losses,
                points: p.points,
                win_rate,
            }
        })
        .collect();

    Ok(Json(leaderboard))
}
