pub mod auth;
pub mod crm;
pub mod reports;
pub mod team;
pub mod expenses;
pub mod dashboard; 
pub mod inventory;

use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
};
use askama::Template;
use tower_cookies::Cookies;

use crate::{
    database::Database,
    middleware::get_current_user,
};

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    user_name: String,
    customer_count: i64,
    team_member_count: i64,
    has_team_access: bool,
    has_inventory_access: bool,
    has_expenses_access: bool,
    has_shipping_access: bool,
    has_api_access: bool,
}

pub async fn dashboard(
    cookies: Cookies,
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Get active customer count (prospect + active status)
    let customer_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM customers WHERE status IN ('prospect', 'active')"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    // Get actual team member count (active users)
    let team_member_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE is_active = true"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);
    
    let template = DashboardTemplate {
        user_name: format!("{} {}", user.first_name, user.last_name),
        customer_count,
        team_member_count,
        has_team_access: user.permissions.contains(&"team:read".to_string()),
        has_inventory_access: user.permissions.contains(&"inventory:read".to_string()),
        has_expenses_access: user.permissions.contains(&"expenses:read".to_string()),
        has_shipping_access: user.permissions.contains(&"shipping:read".to_string()),
        has_api_access: user.permissions.contains(&"api:access".to_string()),
    };
    
    Ok(Html(template.render().unwrap()))
}