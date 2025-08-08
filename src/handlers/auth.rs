use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, Redirect, IntoResponse},
};
use askama::Template;
use serde::Deserialize;
use tower_cookies::{Cookies, Cookie};
use chrono::{Utc, Duration};
use uuid::Uuid;

use crate::{
    database::Database,
    models::{CreateUser, User},
    utils::{create_token, hash_password, verify_password},
};

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: String,
}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    error: String,
}

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    email: String,
    password: String,
    first_name: String,
    last_name: String,
}

pub async fn login_page() -> Html<String> {
    let template = LoginTemplate { 
        error: String::new() 
    };
    Html(template.render().unwrap())
}

pub async fn register_page() -> Html<String> {
    let template = RegisterTemplate { 
        error: String::new() 
    };
    Html(template.render().unwrap())
}

pub async fn login(
    State(db): State<Database>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, (StatusCode, Html<String>)> {
    match authenticate_user(&db, &form.email, &form.password).await {
        Ok(user) => {
            // Create JWT token
            let token = create_token(user.id, user.email.clone())
                .map_err(|_| {
                    let template = LoginTemplate {
                        error: "Authentication failed".to_string(),
                    };
                    (StatusCode::INTERNAL_SERVER_ERROR, Html(template.render().unwrap()))
                })?;
            
            // Create session record in database for additional tracking
            let session_id = Uuid::new_v4();
            let expires_at = Utc::now() + Duration::hours(24);
            
            let _ = sqlx::query!(
                "INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3)",
                session_id,
                user.id,
                expires_at
            )
            .execute(&db)
            .await;
            
            // Update last login
            let _ = sqlx::query!(
                "UPDATE users SET last_login = NOW() WHERE id = $1",
                user.id
            )
            .execute(&db)
            .await;
            
            // Set secure HTTP-only cookie with JWT token
            let cookie = Cookie::build(("auth_token", token))
                .path("/")
                .http_only(true)
                .max_age(time::Duration::hours(24))
                .build();
            
            cookies.add(cookie);
            
            Ok(Redirect::to("/dashboard"))
        }
        Err(_) => {
            let template = LoginTemplate {
                error: "Invalid email or password".to_string(),
            };
            Err((StatusCode::UNAUTHORIZED, Html(template.render().unwrap())))
        }
    }
}

pub async fn logout(cookies: Cookies) -> impl IntoResponse {
    cookies.remove(Cookie::from("auth_token"));
    Redirect::to("/login")
}

pub async fn register(
    State(db): State<Database>,
    Form(form): Form<RegisterForm>,
) -> Result<Redirect, (StatusCode, Html<String>)> {
    let password_hash = hash_password(&form.password)
        .map_err(|_| {
            let template = RegisterTemplate {
                error: "Failed to process password".to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Html(template.render().unwrap()))
        })?;

    let create_user = CreateUser {
        email: form.email,
        password: form.password,
        first_name: form.first_name,
        last_name: form.last_name,
    };

    match create_user_in_db(&db, &create_user, &password_hash).await {
        Ok(_) => Ok(Redirect::to("/login")),
        Err(_) => {
            let template = RegisterTemplate {
                error: "Email already exists or registration failed".to_string(),
            };
            Err((StatusCode::BAD_REQUEST, Html(template.render().unwrap())))
        }
    }
}

async fn authenticate_user(
    db: &Database,
    email: &str,
    password: &str,
) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = $1 AND is_active = true AND is_locked = false"
    )
    .bind(email)
    .fetch_one(db)
    .await?;

    if verify_password(password, &user.password_hash).unwrap_or(false) {
        Ok(user)
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

async fn create_user_in_db(
    db: &Database,
    user_data: &CreateUser,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, first_name, last_name)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(&user_data.email)
    .bind(password_hash)
    .bind(&user_data.first_name)
    .bind(&user_data.last_name)
    .fetch_one(db)
    .await?;

    Ok(user)
}