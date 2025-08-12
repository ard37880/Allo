use axum::{
    extract::State,
    response::{Html, Redirect},
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
    team_member_count: i64, // MODIFIED: Added this field
    has_inventory_access: bool,
    has_team_access: bool,
    has_expenses_access: bool,
    has_shipping_access: bool,
    has_api_access: bool,
}

pub async fn dashboard(
    cookies: Cookies,
    State(db): State<Database>,
) -> Result<Html<String>, Redirect> {
    let current_user = match get_current_user(cookies, &db).await {
        Some(user) => user,
        None => return Err(Redirect::to("/login")),
    };

    // Get active customer count (prospect + active status)
    let customer_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM customers WHERE status IN ('prospect', 'active')"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    // MODIFIED: Added query to get the team member count
    let team_member_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE is_active = true"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let template = DashboardTemplate {
        user_name: format!("{} {}", current_user.first_name, current_user.last_name),
        customer_count,
        team_member_count, // MODIFIED: Passed the value to the template
        has_inventory_access: current_user.permissions.contains(&"inventory:read".to_string()),
        has_team_access: current_user.has_team_read,
        has_expenses_access: current_user.permissions.contains(&"expenses:read".to_string()),
        has_shipping_access: current_user.permissions.contains(&"shipping:read".to_string()),
        has_api_access: current_user.permissions.contains(&"api:access".to_string()),
    };

    Ok(Html(template.render().unwrap()))
}
