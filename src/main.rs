use axum::{
    extract::{State, Json, Path},
    http::StatusCode,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tokio::net::TcpListener;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use tower_http::cors::{CorsLayer, Any};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[derive(Deserialize)]
struct RegisterPayload {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginPayload {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: i32,
    name: String,
}

#[tokio::main]
async fn main() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/users", get(list_users))
        .route("/users/:id", delete(delete_user))
        .with_state(AppState { pool })
        .layer(cors);

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Server running at http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}

//
// REGISTER
//
async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterPayload>,
) -> Result<Json<UserResponse>, StatusCode> {

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let record = sqlx::query(
        "INSERT INTO users (name, email, password_hash)
         VALUES ($1, $2, $3)
         RETURNING id, name"
    )
    .bind(payload.name)
    .bind(payload.email)
    .bind(password_hash)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.constraint() == Some("users_name_key") {
                return StatusCode::CONFLICT;
            }
        }
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(UserResponse {
        id: record.get("id"),
        name: record.get("name"),
    }))
}

//
// LOGIN
//
async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<StatusCode, StatusCode> {

    let record = sqlx::query(
        "SELECT password_hash FROM users WHERE email = $1"
    )
    .bind(payload.email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = record else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let password_hash: String = row.get("password_hash");

    let parsed_hash =
        PasswordHash::new(&password_hash)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let argon2 = Argon2::default();

    argon2
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(StatusCode::OK)
}

//
// LIST USERS
//
async fn list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserResponse>>, StatusCode> {

    let rows = sqlx::query(
        "SELECT id, name FROM users"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let users = rows
        .into_iter()
        .map(|row| UserResponse {
            id: row.get("id"),
            name: row.get("name"),
        })
        .collect();

    Ok(Json(users))
}

//
// DELETE USER
//
async fn delete_user(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<StatusCode, StatusCode> {

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}