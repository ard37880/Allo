use axum::{
    extract::{Form, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use askama::Template;
use serde::Deserialize;
use tower_cookies::Cookies;
use uuid::Uuid;
use chrono::{Utc, NaiveDateTime};

use crate::{
    database::Database,
    models::{Activity, ActivityDisplay, Customer, Contact, Deal},
    middleware::get_current_user,
};

#[derive(Template)]
#[template(path = "crm/activities.html")]
struct ActivitiesTemplate {
    activities: Vec<ActivityDisplay>,
}

#[derive(Template)]
#[template(path = "crm/activity_form.html")]
struct ActivityFormTemplate {
    activity: Option<Activity>,
    customers: Vec<Customer>,
    contacts: Vec<Contact>,
    deals: Vec<Deal>,
    customer_id: Option<Uuid>,
    deal_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ActivityQuery {
    customer_id: Option<Uuid>,
    deal_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ActivityForm {
    customer_id: String, // Changed to String to handle empty values
    contact_id: Option<String>, // Changed to Option<String>
    deal_id: Option<String>, // Changed to Option<String>
    activity_type: String,
    subject: String,
    description: Option<String>,
    activity_date: String, // Changed to String to handle parsing manually
    duration_minutes: Option<i32>,
    completed: Option<String>,
}

pub async fn activities_list(
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let activities = sqlx::query_as::<_, Activity>(
        "SELECT * FROM activities ORDER BY activity_date DESC"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(ActivityDisplay::from)
    .collect();

    let template = ActivitiesTemplate { activities };
    Ok(Html(template.render().unwrap()))
}

pub async fn activity_form(
    State(db): State<Database>,
    Query(query): Query<ActivityQuery>,
) -> Result<Html<String>, StatusCode> {
    let customers = sqlx::query_as::<_, Customer>(
        "SELECT * FROM customers ORDER BY company_name"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let contacts = if let Some(customer_id) = query.customer_id {
        sqlx::query_as::<_, Contact>(
            "SELECT * FROM contacts WHERE customer_id = $1 ORDER BY first_name"
        )
        .bind(customer_id)
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        Vec::new()
    };

    let deals = if let Some(customer_id) = query.customer_id {
        sqlx::query_as::<_, Deal>(
            "SELECT * FROM deals WHERE customer_id = $1 ORDER BY title"
        )
        .bind(customer_id)
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        Vec::new()
    };

    let template = ActivityFormTemplate {
        activity: None,
        customers,
        contacts,
        deals,
        customer_id: query.customer_id,
        deal_id: query.deal_id,
    };
    Ok(Html(template.render().unwrap()))
}

pub async fn create_activity(
    State(db): State<Database>,
    cookies: Cookies,
    Form(form): Form<ActivityForm>,
) -> Result<Redirect, StatusCode> {
    let user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Parse customer_id
    let customer_id = Uuid::parse_str(&form.customer_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse contact_id if provided and not empty
    let contact_id = if let Some(contact_str) = form.contact_id {
        if contact_str.trim().is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&contact_str).map_err(|_| StatusCode::BAD_REQUEST)?)
        }
    } else {
        None
    };

    // Parse deal_id if provided and not empty
    let deal_id = if let Some(deal_str) = form.deal_id {
        if deal_str.trim().is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&deal_str).map_err(|_| StatusCode::BAD_REQUEST)?)
        }
    } else {
        None
    };

    // Parse activity_date
    let activity_date = if form.activity_date.is_empty() {
        Utc::now()
    } else {
        // Parse datetime-local format (YYYY-MM-DDTHH:MM)
        NaiveDateTime::parse_from_str(&form.activity_date, "%Y-%m-%dT%H:%M")
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .and_utc()
    };

    let completed = form.completed.is_some();

    sqlx::query(
        r#"
        INSERT INTO activities (
            customer_id, contact_id, deal_id, activity_type, subject,
            description, activity_date, duration_minutes, completed, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(&customer_id)
    .bind(&contact_id)
    .bind(&deal_id)
    .bind(&form.activity_type)
    .bind(&form.subject)
    .bind(&form.description)
    .bind(&activity_date)
    .bind(&form.duration_minutes)
    .bind(completed)
    .bind(&user.id)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/crm/activities"))
}