use axum::{
    extract::{State, Json, Path},
    http::{StatusCode, HeaderMap},
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use tokio::net::TcpListener;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use jsonwebtoken::{encode, decode, EncodingKey, DecodingKey, Header, Validation};
use tower_http::cors::{CorsLayer, Any};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    jwt_secret: String,
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

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

// BLOG
#[derive(Serialize)]
struct BlogPost {
    id: i32,
    username: String,
    content: String,
}

#[derive(Deserialize)]
struct CreatePost {
    content: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let jwt_secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let pool = loop {
        match PgPool::connect(&database_url).await {
            Ok(pool) => break pool,
            Err(_) => tokio::time::sleep(Duration::from_secs(5)).await,
        }
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/users", get(list_users))
        .route("/users/:id", delete(delete_user))
        .route("/posts", get(get_posts).post(create_post))
        .route("/posts/:id", delete(delete_post))
        .with_state(AppState { pool, jwt_secret })
        .layer(cors);

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

// 🔐 VERIFY JWT
fn verify_token(headers: &HeaderMap, secret: &str) -> Result<String, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth_header.trim_start_matches("Bearer ").trim();

    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(decoded.claims.sub)
}

// REGISTER
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
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(UserResponse {
        id: record.get("id"),
        name: record.get("name"),
    }))
}

// ✅ LOGIN (FIXED)
async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<serde_json::Value>, StatusCode> {

    let record = sqlx::query(
        "SELECT password_hash, name FROM users WHERE email = $1"
    )
    .bind(&payload.email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = record else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let password_hash: String = row.get("password_hash");
    let username: String = row.get("name");

    let parsed_hash = PasswordHash::new(&password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let argon2 = Argon2::default();

    if argon2
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() + 86400;

    let username = username.to_lowercase();

    let claims = Claims {
        sub: username.clone(),
        exp: exp as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 🔥 THIS IS THE CRITICAL FIX
    Ok(Json(json!({
        "token": token,
        "username": username
    })))
}

// USERS (ADMIN)
async fn list_users(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<Vec<UserResponse>>, StatusCode> {

    let username = verify_token(&headers, &state.jwt_secret)?;

    if username != "nigel2" {
        return Err(StatusCode::FORBIDDEN);
    }

    let rows = sqlx::query("SELECT id, name FROM users")
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

// DELETE USER
async fn delete_user(
    headers: HeaderMap,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<StatusCode, StatusCode> {

    let username = verify_token(&headers, &state.jwt_secret)?;

    if username != "nigel2" {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

// GET POSTS
async fn get_posts(
    State(state): State<AppState>,
) -> Result<Json<Vec<BlogPost>>, StatusCode> {

    let rows = sqlx::query(
        "SELECT id, username, content FROM posts ORDER BY id DESC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let posts = rows
        .into_iter()
        .map(|row| BlogPost {
            id: row.get("id"),
            username: row.get("username"),
            content: row.get("content"),
        })
        .collect();

    Ok(Json(posts))
}

// CREATE POST
async fn create_post(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreatePost>,
) -> Result<StatusCode, StatusCode> {

    let username = verify_token(&headers, &state.jwt_secret)?
        .to_lowercase();

    sqlx::query(
        "INSERT INTO posts (username, content) VALUES ($1, $2)"
    )
    .bind(username)
    .bind(payload.content)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

// DELETE POST
async fn delete_post(
    headers: HeaderMap,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<StatusCode, StatusCode> {

    let username = verify_token(&headers, &state.jwt_secret)?
        .to_lowercase();

    let row = sqlx::query(
        "SELECT username FROM posts WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = row else {
        return Err(StatusCode::NOT_FOUND);
    };

    let owner: String = row.get::<String, _>("username").to_lowercase();

    if username != "nigel2" && username != owner {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query("DELETE FROM posts WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}