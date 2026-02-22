use axum::{
    routing::{get, post},
    Router, Json,
    extract::State,
};
use serde::{Serialize, Deserialize};
use sqlx::{PgPool, FromRow};

#[derive(Serialize, FromRow)]
struct User {
    id: i32,
    name: String,
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
}

// GET /users
async fn list_users(
    State(pool): State<PgPool>,
) -> Json<Vec<User>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, name FROM users"
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_else(|_| vec![]);

    Json(users)
}

// POST /users
async fn create_user(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateUser>,
) -> Json<User> {
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name) VALUES ($1) RETURNING id, name"
    )
    .bind(payload.name)
    .fetch_one(&pool)
    .await
    .expect("Failed to insert user");

    Json(user)
}

#[tokio::main]
async fn main() {
    let database_url = "postgres:///axum_test";

    let pool = PgPool::connect(database_url)
        .await
        .expect("Failed to connect to database");

    let app = Router::new()
        .route("/users", get(list_users).post(create_user))
        .with_state(pool);

    println!("Server running at http://127.0.0.1:3000");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}
