mod database;
mod handlers;
mod middleware;
mod models;
mod utils;
mod filters; // Add this line

use axum::{
    body::Bytes,
    extract::DefaultBodyLimit,
    response::Redirect,
    routing::{get, post},
    Router,
};
use std::env;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use dotenvy::dotenv;

use database::{Database, create_database_pool};

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv().ok();
    
    // Initialize logging
    env_logger::init();
    
    // Initialize database
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let db = create_database_pool(&database_url).await
        .expect("Failed to connect to database");
    
    println!("Database connection successful!");
    
    // Build the application router
    let app = create_router(db);
    
    // Get port from environment or use default
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    println!("🚀 Allo server starting on http://{}", addr);
    
    // Start the server
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Custom handler for form data that handles raw body
async fn handle_create_user(
    cookies: tower_cookies::Cookies,
    axum::extract::State(db): axum::extract::State<Database>,
    body: Bytes,
) -> Result<Redirect, axum::http::StatusCode> {
    let body_str = String::from_utf8(body.to_vec())
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    handlers::team::create_user(cookies, axum::extract::State(db), body_str).await
}

async fn handle_update_user(
    cookies: tower_cookies::Cookies,
    axum::extract::Path(user_id): axum::extract::Path<uuid::Uuid>,
    axum::extract::State(db): axum::extract::State<Database>,
    body: Bytes,
) -> Result<Redirect, axum::http::StatusCode> {
    let body_str = String::from_utf8(body.to_vec())
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    handlers::team::update_user(cookies, axum::extract::State(db), axum::extract::Path(user_id), body_str).await
}

async fn handle_create_role(
    cookies: tower_cookies::Cookies,
    axum::extract::State(db): axum::extract::State<Database>,
    body: Bytes,
) -> Result<Redirect, axum::http::StatusCode> {
    let body_str = String::from_utf8(body.to_vec())
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    handlers::team::create_role(cookies, axum::extract::State(db), body_str).await
}

async fn handle_update_role(
    cookies: tower_cookies::Cookies,
    axum::extract::Path(role_id): axum::extract::Path<uuid::Uuid>,
    axum::extract::State(db): axum::extract::State<Database>,
    body: Bytes,
) -> Result<Redirect, axum::http::StatusCode> {
    let body_str = String::from_utf8(body.to_vec())
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    handlers::team::update_role(cookies, axum::extract::State(db), axum::extract::Path(role_id), body_str).await
}

fn create_router(db: Database) -> Router {
    Router::new()
        // Public routes (no authentication required)
        .route("/", get(|| async { Redirect::permanent("/login") }))
        .route("/login", get(handlers::auth::login_page))
        .route("/login", post(handlers::auth::login))
        .route("/register", get(handlers::auth::register_page))
        .route("/register", post(handlers::auth::register))
        .route("/logout", post(handlers::auth::logout))
        
        // Protected routes (authentication required)
        .route("/dashboard", get(handlers::dashboard))
        
        // CRM routes
        .route("/crm", get(handlers::crm::crm_dashboard))
        .route("/crm/customers", get(handlers::crm::customers_list))
        .route("/crm/customers/new", get(handlers::crm::customer_form))
        .route("/crm/customers", post(handlers::crm::create_customer))
        .route("/crm/customers/:id", get(handlers::crm::customer_detail))
        .route("/crm/customers/:id/edit", get(handlers::crm::customer_edit_form))
        .route("/crm/customers/:id", post(handlers::crm::update_customer))
        .route("/crm/customers/:id/delete", get(handlers::crm::delete_customer))
        
        // Contacts
        .route("/crm/contacts", post(handlers::crm::create_contact))
        .route("/crm/customers/:customer_id/contacts/:contact_id/delete", 
               get(handlers::crm::delete_contact))
        
        // Deals routes
        .route("/crm/deals", get(handlers::crm::deals_list))
        .route("/crm/deals/new", get(handlers::crm::deal_form))
        .route("/crm/deals", post(handlers::crm::create_deal))
        .route("/crm/deals/:id", get(handlers::crm::deal_detail))
        .route("/crm/deals/:id/edit", get(handlers::crm::deal_edit_form))
        .route("/crm/deals/:id", post(handlers::crm::update_deal))
        
        // Activities routes
        .route("/crm/activities", get(handlers::crm::activities_list))
        .route("/crm/activities/new", get(handlers::crm::activity_form))
        .route("/crm/activities", post(handlers::crm::create_activity))
        
        // Reports routes
        .route("/crm/reports", get(handlers::reports::reports_list))
        
        // Team management routes
        .route("/team", get(handlers::team::team_dashboard))
        .route("/team/users", get(handlers::team::users_list))
        .route("/team/users/new", get(handlers::team::user_form))
        .route("/team/users", post(handle_create_user)) // Use custom handler
        .route("/team/users/:id/edit", get(handlers::team::user_edit_form))
        .route("/team/users/:id", post(handle_update_user)) // Use custom handler
        .route("/team/users/:id/lock", get(handlers::team::lock_user))
        .route("/team/users/:id/unlock", get(handlers::team::unlock_user))
        .route("/team/users/:id/delete", get(handlers::team::delete_user))
        
        // Roles routes
        .route("/team/roles", get(handlers::team::roles_list))
        .route("/team/roles/new", get(handlers::team::role_form))
        .route("/team/roles", post(handle_create_role)) // Use custom handler
        .route("/team/roles/:id/edit", get(handlers::team::role_edit_form))
        .route("/team/roles/:id", post(handle_update_role)) // Use custom handler
        .route("/team/roles/:id/delete", get(handlers::team::delete_role))
        
        // API routes
        .route("/api/customers/:id/contacts", get(handlers::crm::get_customer_contacts))
        
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        
        // Middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CookieManagerLayer::new())
                .layer(CorsLayer::permissive())
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB
        )
        .with_state(db)
}