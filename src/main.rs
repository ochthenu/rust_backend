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

<<<<<<< HEAD
use jsonwebtoken::{encode, decode, EncodingKey, DecodingKey, Header, Validation};

=======
use jsonwebtoken::{encode, EncodingKey, Header};
>>>>>>> 6c33cd7 (update main.rs debug)
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

#[tokio::main]
async fn main() {
    println!("🚀 Starting backend (new version!) 🔥"); // <- DEBUG LINE

<<<<<<< HEAD
    let jwt_secret =
        std::env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set");

=======
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let jwt_secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    // Retry DB connection
>>>>>>> 6c33cd7 (update main.rs debug)
    let pool = loop {
        match PgPool::connect(&database_url).await {
            Ok(pool) => {
                println!("✅ Connected to database");
                break pool;
            }
            Err(e) => {
                eprintln!("❌ DB connection failed, retrying: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    };

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
<<<<<<< HEAD
        .route("/users", get(list_users))
        .route("/users/:id", delete(delete_user))
=======
        .route("/users", get(list_users))        // public
        .route("/users/:id", delete(delete_user)) // delete
>>>>>>> 6c33cd7 (update main.rs debug)
        .with_state(AppState { pool, jwt_secret })
        .layer(cors);

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("🚀 Server running at http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}

//
// 🔐 JWT VERIFY HELPER
//
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

//
// REGISTER
//
async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterPayload>,
) -> Result<Json<UserResponse>, StatusCode> {

    println!("📝 REGISTER HIT: {}", payload.email);

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| {
            eprintln!("❌ Hash error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
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
        eprintln!("❌ Register error: {}", e);
<<<<<<< HEAD
=======
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.constraint() == Some("users_name_key") {
                return StatusCode::CONFLICT;
            }
        }
>>>>>>> 6c33cd7 (update main.rs debug)
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    println!("✅ USER REGISTERED");

    Ok(Json(UserResponse {
        id: record.get("id"),
        name: record.get("name"),
    }))
}

//
<<<<<<< HEAD
// LOGIN (FIXED + DEBUG)
=======
// LOGIN (JWT)
>>>>>>> 6c33cd7 (update main.rs debug)
//
async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<serde_json::Value>, StatusCode> {

    println!("🔥 LOGIN HIT: {}", payload.email);

    let record = sqlx::query(
        "SELECT password_hash, name FROM users WHERE email = $1"
    )
    .bind(&payload.email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        eprintln!("❌ DB ERROR: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    println!("✅ DB QUERY DONE");

    let Some(row) = record else {
        println!("❌ USER NOT FOUND");
        return Err(StatusCode::UNAUTHORIZED);
    };

    let password_hash: String = row.get("password_hash");
    let username: String = row.get("name");

    println!("🔐 VERIFYING PASSWORD");

    let parsed_hash = PasswordHash::new(&password_hash)
        .map_err(|e| {
            eprintln!("❌ HASH PARSE ERROR: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let argon2 = Argon2::default();

    if argon2
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        println!("❌ INVALID PASSWORD");
        return Err(StatusCode::UNAUTHORIZED);
    }

    println!("✅ PASSWORD VERIFIED");

    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() + 86400;

    let claims = Claims {
        sub: username,
        exp: exp as usize,
    };

    println!("🔑 GENERATING TOKEN");

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
<<<<<<< HEAD
        eprintln!("❌ JWT ERROR: {}", e);
=======
        eprintln!("❌ JWT error: {}", e);
>>>>>>> 6c33cd7 (update main.rs debug)
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    println!("✅ LOGIN SUCCESS");

    Ok(Json(json!({ "token": token })))
}

//
// 🔐 LIST USERS
//
async fn list_users(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<Vec<UserResponse>>, StatusCode> {

<<<<<<< HEAD
    let username = verify_token(&headers, &state.jwt_secret)?;

    if username != "nigel2" {
        return Err(StatusCode::FORBIDDEN);
    }
=======
    println!("📢 list_users called!"); // <- DEBUG LINE
>>>>>>> 6c33cd7 (update main.rs debug)

    let rows = sqlx::query("SELECT id, name FROM users")
        .fetch_all(&state.pool)
        .await
<<<<<<< HEAD
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
=======
        .map_err(|e| {
            eprintln!("❌ List users error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
>>>>>>> 6c33cd7 (update main.rs debug)

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
// 🔐 DELETE USER
//
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
<<<<<<< HEAD
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
=======
        .map_err(|e| {
            eprintln!("❌ Delete error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
>>>>>>> 6c33cd7 (update main.rs debug)

    Ok(StatusCode::OK)
}