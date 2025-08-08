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
    let current_user = get_current_user(cookies, &db).await;
    
    if current_user.is_none() {
        return Err(Redirect::to("/login"));
    }
    
    let current_user = current_user.unwrap();

    let customer_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM customers")
        .fetch_one(&db)
        .await
        .unwrap_or(0);

    let template = DashboardTemplate {
        user_name: format!("{} {}", current_user.first_name, current_user.last_name),
        customer_count,
        has_inventory_access: current_user.permissions.contains(&"inventory:read".to_string()),
        has_team_access: current_user.has_team_read,
        has_expenses_access: current_user.permissions.contains(&"expenses:read".to_string()),
        has_shipping_access: current_user.permissions.contains(&"shipping:read".to_string()),
        has_api_access: current_user.permissions.contains(&"api:access".to_string()),
    };

    Ok(Html(template.render().unwrap()))
}