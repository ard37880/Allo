use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Html,
};
use askama::Template;
use serde::Deserialize;
use chrono::{DateTime, Utc, NaiveDate};
use uuid::Uuid;
use sqlx::Row;

use crate::{
    database::Database,
    models::{Customer, User},
};

#[derive(Template)]
#[template(path = "crm/reports.html")]
struct ReportsTemplate {
    reports: Vec<ReportEntry>,
    customers: Vec<Customer>,
    users: Vec<User>,
    selected_customer: Option<Uuid>,
    selected_user: Option<Uuid>,
    selected_date_from: String,
    selected_date_to: String,
}

#[derive(Deserialize)]
pub struct ReportFilters {
    customer_id: Option<String>,
    user_id: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
}

#[derive(Debug)]
pub struct ReportEntry {
    pub id: Uuid,
    pub action: String,
    pub subject: String,
    pub description: String,
    pub user_name: String,
    pub customer_name: String,
    pub activity_date: DateTime<Utc>,
    pub activity_type: String,
}

pub async fn reports_list(
    query: Query<ReportFilters>,
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    // Parse customer_id if provided and not empty
    let customer_id = if let Some(customer_str) = &query.customer_id {
        if customer_str.trim().is_empty() {
            None
        } else {
            Some(Uuid::parse_str(customer_str).map_err(|_| StatusCode::BAD_REQUEST)?)
        }
    } else {
        None
    };

    // Parse user_id if provided and not empty
    let user_id = if let Some(user_str) = &query.user_id {
        if user_str.trim().is_empty() {
            None
        } else {
            Some(Uuid::parse_str(user_str).map_err(|_| StatusCode::BAD_REQUEST)?)
        }
    } else {
        None
    };

    // Get all customers for filter dropdown
    let customers = sqlx::query_as::<_, Customer>(
        "SELECT * FROM customers ORDER BY company_name"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get all users for filter dropdown
    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY first_name, last_name"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build dynamic query based on filters
    let mut conditions = Vec::new();
    let mut bind_count = 1;

    if customer_id.is_some() {
        conditions.push(format!("a.customer_id = ${}", bind_count));
        bind_count += 1;
    }

    if user_id.is_some() {
        conditions.push(format!("a.created_by = ${}", bind_count));
        bind_count += 1;
    }

    if query.date_from.is_some() && !query.date_from.as_ref().unwrap().trim().is_empty() {
        conditions.push(format!("DATE(a.activity_date) >= ${}", bind_count));
        bind_count += 1;
    }

    if query.date_to.is_some() && !query.date_to.as_ref().unwrap().trim().is_empty() {
        conditions.push(format!("DATE(a.activity_date) <= ${}", bind_count));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let query_sql = format!(
        r#"
        SELECT 
            a.id,
            a.subject,
            COALESCE(a.description, '') as description,
            COALESCE(CONCAT(u.first_name, ' ', u.last_name), 'Unknown User') as user_name,
            COALESCE(c.company_name, 'Unknown Customer') as customer_name,
            a.activity_date,
            a.activity_type
        FROM activities a
        LEFT JOIN users u ON a.created_by = u.id
        LEFT JOIN customers c ON a.customer_id = c.id
        {}
        ORDER BY a.activity_date DESC
        LIMIT 100
        "#,
        where_clause
    );

    // Build query with parameters
    let mut sqlx_query = sqlx::query(&query_sql);

    if let Some(cid) = customer_id {
        sqlx_query = sqlx_query.bind(cid);
    }

    if let Some(uid) = user_id {
        sqlx_query = sqlx_query.bind(uid);
    }

    if let Some(date_from) = &query.date_from {
        if !date_from.trim().is_empty() {
            if let Ok(parsed_date) = NaiveDate::parse_from_str(date_from, "%Y-%m-%d") {
                sqlx_query = sqlx_query.bind(parsed_date);
            }
        }
    }

    if let Some(date_to) = &query.date_to {
        if !date_to.trim().is_empty() {
            if let Ok(parsed_date) = NaiveDate::parse_from_str(date_to, "%Y-%m-%d") {
                sqlx_query = sqlx_query.bind(parsed_date);
            }
        }
    }

    let rows = sqlx_query
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut reports = Vec::new();
    for row in rows {
        let id: Uuid = row.try_get("id").unwrap_or_default();
        let subject: String = row.try_get("subject").unwrap_or_default();
        let description: String = row.try_get("description").unwrap_or_default();
        let user_name: String = row.try_get("user_name").unwrap_or_else(|_| "Unknown User".to_string());
        let customer_name: String = row.try_get("customer_name").unwrap_or_else(|_| "Unknown Customer".to_string());
        let activity_date: DateTime<Utc> = row.try_get("activity_date").unwrap_or_else(|_| Utc::now());
        let activity_type: String = row.try_get("activity_type").unwrap_or_default();

        reports.push(ReportEntry {
            id,
            action: format!("{} - {}", activity_type.to_uppercase(), subject),
            subject,
            description,
            user_name,
            customer_name,
            activity_date,
            activity_type,
        });
    }

    let template = ReportsTemplate {
        reports,
        customers,
        users,
        selected_customer: customer_id,
        selected_user: user_id,
        selected_date_from: query.date_from.clone().unwrap_or_default(),
        selected_date_to: query.date_to.clone().unwrap_or_default(),
    };

    Ok(Html(template.render().unwrap()))
}