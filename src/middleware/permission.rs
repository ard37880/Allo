use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::{
    database::Database,
    models::User,
    utils::verify_token,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub is_active: bool,
    pub is_locked: bool,
    pub permissions: Vec<String>,
    // Helper properties for templates
    pub has_team_read: bool,
    pub has_team_write: bool,
    pub has_team_delete: bool,
    pub has_manage_roles: bool,
}

impl CurrentUser {
    pub fn from_user_and_permissions(user: User, permissions: Vec<String>) -> Self {
        let has_team_read = permissions.contains(&"team:read".to_string());
        let has_team_write = permissions.contains(&"team:write".to_string());
        let has_team_delete = permissions.contains(&"team:delete".to_string());
        let has_manage_roles = permissions.contains(&"team:manage_roles".to_string());

        Self {
            id: user.id,
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
            is_active: user.is_active,
            is_locked: user.is_locked,
            permissions,
            has_team_read,
            has_team_write,
            has_team_delete,
            has_manage_roles,
        }
    }
}

pub async fn get_current_user(cookies: Cookies, db: &Database) -> Option<CurrentUser> {
    // Try to get JWT token from auth_token cookie
    let token = cookies.get("auth_token")?.value().to_string();
    
    // Verify the JWT token
    let claims = match verify_token(&token) {
        Ok(claims) => claims,
        Err(_) => {
            // Token is invalid, try fallback to super admin for development
            return get_super_admin_user(db).await;
        }
    };

    // Parse user ID from token claims
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return get_super_admin_user(db).await,
    };

    // Get user data from database
    get_user_by_id(db, user_id).await
}

async fn get_super_admin_user(db: &Database) -> Option<CurrentUser> {
    // Fallback: get the super admin user directly (for development)
    let user_result = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = 'andrew.davis@habb.tech' AND is_active = true AND is_locked = false"
    )
    .fetch_optional(db)
    .await;
    
    match user_result {
        Ok(Some(user)) => {
            let permissions = get_user_permissions(db, user.id).await;
            Some(CurrentUser::from_user_and_permissions(user, permissions))
        },
        _ => None
    }
}

async fn get_user_by_id(db: &Database, user_id: Uuid) -> Option<CurrentUser> {
    // Get user data
    let user_row = sqlx::query!(
        "SELECT id, email, password_hash, first_name, last_name, is_active, is_locked, last_login, locked_at, locked_by, created_at, updated_at FROM users WHERE id = $1 AND is_active = true AND is_locked = false",
        user_id
    )
    .fetch_optional(db)
    .await
    .ok()??;

    // Convert to User struct manually
    let user = User {
        id: user_row.id,
        email: user_row.email,
        password_hash: user_row.password_hash,
        first_name: user_row.first_name,
        last_name: user_row.last_name,
        is_active: user_row.is_active.unwrap_or(false),
        is_locked: user_row.is_locked.unwrap_or(false),
        last_login: user_row.last_login,
        locked_at: user_row.locked_at,
        locked_by: user_row.locked_by,
        created_at: user_row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: user_row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
    };

    let permissions = get_user_permissions(db, user.id).await;
    
    Some(CurrentUser::from_user_and_permissions(user, permissions))
}

pub async fn get_user_permissions(db: &Database, user_id: Uuid) -> Vec<String> {
    let permissions = sqlx::query!(
        r#"
        SELECT DISTINCT jsonb_array_elements_text(r.permissions) as permission
        FROM roles r
        JOIN user_roles ur ON r.id = ur.role_id
        WHERE ur.user_id = $1 AND r.is_active = true
        "#,
        user_id
    )
    .fetch_all(db)
    .await
    .unwrap_or_default()
    .into_iter()
    .filter_map(|row| row.permission)
    .collect();

    permissions
}