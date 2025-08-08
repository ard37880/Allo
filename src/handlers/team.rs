use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use askama::Template;
use serde::Deserialize;
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::{
    database::Database,
    models::{User, Role, RoleDisplay, UserWithRoles, get_all_permissions, Permission},
    middleware::{get_current_user, CurrentUser},
    utils::hash_password,
};

#[derive(Template)]
#[template(path = "team/dashboard.html")]
struct TeamDashboardTemplate {
    user_count: i64,
    role_count: i64,
    locked_user_count: i64,
    recent_activities: Vec<AuditLogDisplay>,
    current_user: CurrentUser,
}

#[derive(Template)]
#[template(path = "team/users.html")]
struct UsersTemplate {
    users: Vec<UserWithRoles>,
    current_user: CurrentUser,
}

#[derive(Template)]
#[template(path = "team/user_form.html")]
struct UserFormTemplate {
    user: Option<UserWithRoles>,
    roles: Vec<RoleDisplay>,
    error: String,
    current_user: CurrentUser,
}

#[derive(Template)]
#[template(path = "team/roles.html")]
struct RolesTemplate {
    roles: Vec<RoleDisplay>,
    current_user: CurrentUser,
}

#[derive(Template)]
#[template(path = "team/role_form.html")]
struct RoleFormTemplate {
    role: Option<RoleDisplay>,
    permissions: Vec<Permission>,
    error: String,
    current_user: CurrentUser,
    role_permissions: Vec<String>,
}

#[derive(Deserialize)]
pub struct UserForm {
    email: String,
    password: Option<String>,
    first_name: String,
    last_name: String,
    role_ids: Vec<String>,
    is_active: Option<String>,
}

#[derive(Deserialize)]
pub struct RoleForm {
    name: String,
    description: String,
    permissions: Vec<String>,
    is_active: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuditLogDisplay {
    pub id: Uuid,
    pub user_name: String,
    pub action: String,
    pub resource_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// Team Dashboard
pub async fn team_dashboard(
    cookies: Cookies,
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_team_read {
        return Err(StatusCode::FORBIDDEN);
    }

    let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&db)
        .await
        .unwrap_or(0);

    let role_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM roles WHERE is_active = true")
        .fetch_one(&db)
        .await
        .unwrap_or(0);

    let locked_user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE is_locked = true")
        .fetch_one(&db)
        .await
        .unwrap_or(0);

    // Get recent audit activities - simplified query to avoid IpAddr issues
    let recent_activities = vec![]; // Simplified for now to avoid the IpAddr issue

    let template = TeamDashboardTemplate {
        user_count,
        role_count,
        locked_user_count,
        recent_activities,
        current_user,
    };
    
    Ok(Html(template.render().unwrap()))
}

// Users Management
pub async fn users_list(
    cookies: Cookies,
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_team_read {
        return Err(StatusCode::FORBIDDEN);
    }

    let users = get_users_with_roles(&db).await.unwrap_or_default();

    let template = UsersTemplate { users, current_user };
    Ok(Html(template.render().unwrap()))
}

pub async fn user_form(
    cookies: Cookies,
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_team_write {
        return Err(StatusCode::FORBIDDEN);
    }

    let roles = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE is_active = true ORDER BY name")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(RoleDisplay::from)
        .collect();

    let template = UserFormTemplate {
        user: None,
        roles,
        error: String::new(),
        current_user,
    };
    Ok(Html(template.render().unwrap()))
}

pub async fn user_edit_form(
    cookies: Cookies,
    State(db): State<Database>,
    Path(user_id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_team_write {
        return Err(StatusCode::FORBIDDEN);
    }

    let user = get_user_with_roles(&db, user_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let roles = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE is_active = true ORDER BY name")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(RoleDisplay::from)
        .collect();

    let template = UserFormTemplate {
        user: Some(user),
        roles,
        error: String::new(),
        current_user,
    };
    Ok(Html(template.render().unwrap()))
}

pub async fn create_user(
    cookies: Cookies,
    State(db): State<Database>,
    Form(form): Form<UserForm>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_team_write {
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate password is provided for new users
    let password = form.password.ok_or(StatusCode::BAD_REQUEST)?;
    if password.len() < 6 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let password_hash = hash_password(&password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let is_active = form.is_active.is_some();

    // Create user
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, first_name, last_name, is_active)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(&form.email)
    .bind(&password_hash)
    .bind(&form.first_name)
    .bind(&form.last_name)
    .bind(is_active)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Assign roles
    for role_id_str in form.role_ids {
        if let Ok(role_id) = Uuid::parse_str(&role_id_str) {
            let _ = sqlx::query(
                "INSERT INTO user_roles (user_id, role_id, assigned_by) VALUES ($1, $2, $3)"
            )
            .bind(user.id)
            .bind(role_id)
            .bind(current_user.id)
            .execute(&db)
            .await;
        }
    }

    // Create audit log
    let _ = create_audit_log(
        &db,
        current_user.id,
        "create".to_string(),
        "user".to_string(),
        Some(user.id),
        None,
        Some(serde_json::json!({
            "email": form.email,
            "first_name": form.first_name,
            "last_name": form.last_name,
            "is_active": is_active
        })),
    ).await;

    Ok(Redirect::to("/team/users"))
}

pub async fn update_user(
    cookies: Cookies,
    State(db): State<Database>,
    Path(user_id): Path<Uuid>,
    Form(form): Form<UserForm>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_team_write {
        return Err(StatusCode::FORBIDDEN);
    }

    let is_active = form.is_active.is_some();

    // Handle password update properly
    if let Some(password) = &form.password {
        if !password.is_empty() && password.len() >= 6 {
            let password_hash = hash_password(password)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            // Update with password
            sqlx::query(
                "UPDATE users SET email = $1, first_name = $2, last_name = $3, is_active = $4, password_hash = $5, updated_at = NOW() WHERE id = $6"
            )
            .bind(&form.email)
            .bind(&form.first_name)
            .bind(&form.last_name)
            .bind(is_active)
            .bind(&password_hash)
            .bind(user_id)
            .execute(&db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        } else {
            // Update without password
            sqlx::query(
                "UPDATE users SET email = $1, first_name = $2, last_name = $3, is_active = $4, updated_at = NOW() WHERE id = $5"
            )
            .bind(&form.email)
            .bind(&form.first_name)
            .bind(&form.last_name)
            .bind(is_active)
            .bind(user_id)
            .execute(&db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    } else {
        // Update without password
        sqlx::query(
            "UPDATE users SET email = $1, first_name = $2, last_name = $3, is_active = $4, updated_at = NOW() WHERE id = $5"
        )
        .bind(&form.email)
        .bind(&form.first_name)
        .bind(&form.last_name)
        .bind(is_active)
        .bind(user_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Update roles - remove existing and add new ones
    sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
        .bind(user_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for role_id_str in form.role_ids {
        if let Ok(role_id) = Uuid::parse_str(&role_id_str) {
            let _ = sqlx::query(
                "INSERT INTO user_roles (user_id, role_id, assigned_by) VALUES ($1, $2, $3)"
            )
            .bind(user_id)
            .bind(role_id)
            .bind(current_user.id)
            .execute(&db)
            .await;
        }
    }

    // Create audit log
    let _ = create_audit_log(
        &db,
        current_user.id,
        "update".to_string(),
        "user".to_string(),
        Some(user_id),
        None,
        Some(serde_json::json!({
            "email": form.email,
            "first_name": form.first_name,
            "last_name": form.last_name,
            "is_active": is_active
        })),
    ).await;

    Ok(Redirect::to("/team/users"))
}

pub async fn lock_user(
    cookies: Cookies,
    State(db): State<Database>,
    Path(user_id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_team_write {
        return Err(StatusCode::FORBIDDEN);
    }

    // Prevent users from locking themselves
    if current_user.id == user_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query(
        "UPDATE users SET is_locked = true, locked_at = NOW(), locked_by = $1 WHERE id = $2"
    )
    .bind(current_user.id)
    .bind(user_id)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create audit log
    let _ = create_audit_log(
        &db,
        current_user.id,
        "lock".to_string(),
        "user".to_string(),
        Some(user_id),
        None,
        Some(serde_json::json!({"locked": true})),
    ).await;

    Ok(Redirect::to("/team/users"))
}

pub async fn unlock_user(
    cookies: Cookies,
    State(db): State<Database>,
    Path(user_id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_team_write {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query(
        "UPDATE users SET is_locked = false, locked_at = NULL, locked_by = NULL WHERE id = $1"
    )
    .bind(user_id)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create audit log
    let _ = create_audit_log(
        &db,
        current_user.id,
        "unlock".to_string(),
        "user".to_string(),
        Some(user_id),
        None,
        Some(serde_json::json!({"locked": false})),
    ).await;

    Ok(Redirect::to("/team/users"))
}

pub async fn delete_user(
    cookies: Cookies,
    State(db): State<Database>,
    Path(user_id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_team_delete {
        return Err(StatusCode::FORBIDDEN);
    }

    // Prevent users from deleting themselves
    if current_user.id == user_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get user info for audit log
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&db)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Delete user (cascade will handle user_roles)
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create audit log
    let _ = create_audit_log(
        &db,
        current_user.id,
        "delete".to_string(),
        "user".to_string(),
        Some(user_id),
        Some(serde_json::json!({
            "email": user.email,
            "first_name": user.first_name,
            "last_name": user.last_name
        })),
        None,
    ).await;

    Ok(Redirect::to("/team/users"))
}

// Roles Management
pub async fn roles_list(
    cookies: Cookies,
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_manage_roles {
        return Err(StatusCode::FORBIDDEN);
    }

    let roles = sqlx::query_as::<_, Role>("SELECT * FROM roles ORDER BY name")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(RoleDisplay::from)
        .collect();

    let template = RolesTemplate { roles, current_user };
    Ok(Html(template.render().unwrap()))
}

pub async fn role_form(
    cookies: Cookies,
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_manage_roles {
        return Err(StatusCode::FORBIDDEN);
    }

    let permissions = get_all_permissions();

    let template = RoleFormTemplate {
        role: None,
        permissions,
        error: String::new(),
        current_user,
        role_permissions: vec![], // Empty for new role
    };
    Ok(Html(template.render().unwrap()))
}

pub async fn role_edit_form(
    cookies: Cookies,
    State(db): State<Database>,
    Path(role_id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_manage_roles {
        return Err(StatusCode::FORBIDDEN);
    }

    let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = $1")
        .bind(role_id)
        .fetch_one(&db)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let permissions = get_all_permissions();
    let role_permissions = role.permissions.0.clone();

    let template = RoleFormTemplate {
        role: Some(RoleDisplay::from(role)),
        permissions,
        error: String::new(),
        current_user,
        role_permissions, // Pass the role's permissions for checking
    };
    Ok(Html(template.render().unwrap()))
}

pub async fn create_role(
    cookies: Cookies,
    State(db): State<Database>,
    Form(form): Form<RoleForm>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_manage_roles {
        return Err(StatusCode::FORBIDDEN);
    }

    let is_active = form.is_active.is_some();
    let permissions_json = serde_json::to_value(&form.permissions)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let role = sqlx::query_as::<_, Role>(
        r#"
        INSERT INTO roles (name, description, permissions, is_active, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(&form.name)
    .bind(if form.description.is_empty() { None } else { Some(&form.description) })
    .bind(permissions_json)
    .bind(is_active)
    .bind(current_user.id)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create audit log
    let _ = create_audit_log(
        &db,
        current_user.id,
        "create".to_string(),
        "role".to_string(),
        Some(role.id),
        None,
        Some(serde_json::json!({
            "name": form.name,
            "description": form.description,
            "permissions": form.permissions,
            "is_active": is_active
        })),
    ).await;

    Ok(Redirect::to("/team/roles"))
}

pub async fn update_role(
    cookies: Cookies,
    State(db): State<Database>,
    Path(role_id): Path<Uuid>,
    Form(form): Form<RoleForm>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_manage_roles {
        return Err(StatusCode::FORBIDDEN);
    }

    let is_active = form.is_active.is_some();
    let permissions_json = serde_json::to_value(&form.permissions)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        r#"
        UPDATE roles SET 
            name = $1, 
            description = $2, 
            permissions = $3, 
            is_active = $4, 
            updated_at = NOW()
        WHERE id = $5
        "#,
    )
    .bind(&form.name)
    .bind(if form.description.is_empty() { None } else { Some(&form.description) })
    .bind(permissions_json)
    .bind(is_active)
    .bind(role_id)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create audit log
    let _ = create_audit_log(
        &db,
        current_user.id,
        "update".to_string(),
        "role".to_string(),
        Some(role_id),
        None,
        Some(serde_json::json!({
            "name": form.name,
            "description": form.description,
            "permissions": form.permissions,
            "is_active": is_active
        })),
    ).await;

    Ok(Redirect::to("/team/roles"))
}

pub async fn delete_role(
    cookies: Cookies,
    State(db): State<Database>,
    Path(role_id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.has_manage_roles {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if role is assigned to any users
    let user_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM user_roles WHERE role_id = $1"
    )
    .bind(role_id)
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    if user_count > 0 {
        return Err(StatusCode::CONFLICT); // Cannot delete role with assigned users
    }

    // Get role info for audit log
    let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = $1")
        .bind(role_id)
        .fetch_one(&db)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Delete role
    sqlx::query("DELETE FROM roles WHERE id = $1")
        .bind(role_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create audit log
    let _ = create_audit_log(
        &db,
        current_user.id,
        "delete".to_string(),
        "role".to_string(),
        Some(role_id),
        Some(serde_json::json!({
            "name": role.name,
            "description": role.description,
            "permissions": role.permissions.0
        })),
        None,
    ).await;

    Ok(Redirect::to("/team/roles"))
}

// Helper functions
async fn get_users_with_roles(db: &Database) -> Result<Vec<UserWithRoles>, sqlx::Error> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY first_name, last_name")
        .fetch_all(db)
        .await?;

    let mut users_with_roles = Vec::new();

    for user in users {
        let roles = sqlx::query_as::<_, Role>(
            r#"
            SELECT r.* FROM roles r
            JOIN user_roles ur ON r.id = ur.role_id
            WHERE ur.user_id = $1
            ORDER BY r.name
            "#
        )
        .bind(user.id)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(RoleDisplay::from)
        .collect::<Vec<_>>();

        let permissions = get_user_permissions_from_roles(&roles);

        users_with_roles.push(UserWithRoles {
            id: user.id,
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
            is_active: user.is_active,
            is_locked: user.is_locked,
            last_login: user.last_login,
            locked_at: user.locked_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
            roles,
            permissions,
        });
    }

    Ok(users_with_roles)
}

async fn get_user_with_roles(db: &Database, user_id: Uuid) -> Result<UserWithRoles, sqlx::Error> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await?;

    let roles = sqlx::query_as::<_, Role>(
        r#"
        SELECT r.* FROM roles r
        JOIN user_roles ur ON r.id = ur.role_id
        WHERE ur.user_id = $1
        ORDER BY r.name
        "#
    )
    .bind(user.id)
    .fetch_all(db)
    .await?
    .into_iter()
    .map(RoleDisplay::from)
    .collect::<Vec<_>>();

    let permissions = get_user_permissions_from_roles(&roles);

    Ok(UserWithRoles {
        id: user.id,
        email: user.email,
        first_name: user.first_name,
        last_name: user.last_name,
        is_active: user.is_active,
        is_locked: user.is_locked,
        last_login: user.last_login,
        locked_at: user.locked_at,
        created_at: user.created_at,
        updated_at: user.updated_at,
        roles,
        permissions,
    })
}

fn get_user_permissions_from_roles(roles: &[RoleDisplay]) -> Vec<String> {
    let mut permissions = Vec::new();
    for role in roles {
        permissions.extend(role.permissions.clone());
    }
    permissions.sort();
    permissions.dedup();
    permissions
}

async fn create_audit_log(
    db: &Database,
    user_id: Uuid,
    action: String,
    resource_type: String,
    resource_id: Option<Uuid>,
    old_values: Option<serde_json::Value>,
    new_values: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (user_id, action, resource_type, resource_id, old_values, new_values)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(old_values)
    .bind(new_values)
    .execute(db)
    .await?;

    Ok(())
}

// Remove the login function from this file - it should be in auth.rs
// This was causing the compilation errors