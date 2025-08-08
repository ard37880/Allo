use axum::{
    extract::{Form, Query, Path, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use askama::Template;
use serde::Deserialize;
use tower_cookies::Cookies;
use uuid::Uuid;
use chrono::NaiveDate;

use crate::{
    database::Database,
    models::{Deal, DealDisplay, Customer, Contact},
    middleware::get_current_user,
};

#[derive(Template)]
#[template(path = "crm/deals.html")]
struct DealsTemplate {
    deals: Vec<DealDisplay>,
}

#[derive(Template)]
#[template(path = "crm/deal_form.html")]
struct DealFormTemplate {
    deal: Option<Deal>,
    customers: Vec<Customer>,
    contacts: Vec<Contact>,
    customer_id: Option<Uuid>,
}

#[derive(Template)]
#[template(path = "crm/deal_detail.html")]
struct DealDetailTemplate {
    deal: DealDisplay,
    customer: Customer,
    contact: Option<Contact>,
}

#[derive(Deserialize)]
pub struct DealQuery {
    customer_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct DealForm {
    customer_id: String, // Changed to String to handle empty values
    contact_id: Option<String>, // Changed to Option<String>
    title: String,
    description: Option<String>,
    value: Option<rust_decimal::Decimal>,
    currency: String,
    stage: String,
    expected_close_date: Option<NaiveDate>,
}

pub async fn deals_list(
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let deals = sqlx::query_as::<_, Deal>(
        "SELECT * FROM deals ORDER BY created_at DESC"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(DealDisplay::from)
    .collect();

    let template = DealsTemplate { deals };
    Ok(Html(template.render().unwrap()))
}

pub async fn deal_form(
    State(db): State<Database>,
    Query(query): Query<DealQuery>,
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

    let template = DealFormTemplate {
        deal: None,
        customers,
        contacts,
        customer_id: query.customer_id,
    };
    Ok(Html(template.render().unwrap()))
}

pub async fn deal_detail(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let deal = sqlx::query_as::<_, Deal>(
        "SELECT * FROM deals WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    let customer = sqlx::query_as::<_, Customer>(
        "SELECT * FROM customers WHERE id = $1"
    )
    .bind(deal.customer_id)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let contact = if let Some(contact_id) = deal.contact_id {
        sqlx::query_as::<_, Contact>(
            "SELECT * FROM contacts WHERE id = $1"
        )
        .bind(contact_id)
        .fetch_optional(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        None
    };

    let template = DealDetailTemplate {
        deal: DealDisplay::from(deal),
        customer,
        contact,
    };
    
    Ok(Html(template.render().unwrap()))
}

pub async fn deal_edit_form(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let deal = sqlx::query_as::<_, Deal>(
        "SELECT * FROM deals WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    let customers = sqlx::query_as::<_, Customer>(
        "SELECT * FROM customers ORDER BY company_name"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let contacts = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts WHERE customer_id = $1 ORDER BY first_name"
    )
    .bind(deal.customer_id)
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = DealFormTemplate {
        deal: Some(deal),
        customers,
        contacts,
        customer_id: None,
    };
    Ok(Html(template.render().unwrap()))
}

pub async fn create_deal(
    State(db): State<Database>,
    cookies: Cookies,
    Form(form): Form<DealForm>,
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

    // Default probability based on stage
    let probability = match form.stage.as_str() {
        "prospect" => 25,
        "negotiation" => 75,
        "closed_won" => 100,
        "closed_lost" => 0,
        _ => 50,
    };

    let deal = sqlx::query_as::<_, Deal>(
        r#"
        INSERT INTO deals (
            customer_id, contact_id, title, description, value,
            currency, stage, probability, expected_close_date, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(&customer_id)
    .bind(&contact_id)
    .bind(&form.title)
    .bind(&form.description)
    .bind(&form.value)
    .bind(&form.currency)
    .bind(&form.stage)
    .bind(&probability)
    .bind(&form.expected_close_date)
    .bind(&user.id)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/crm/deals/{}", deal.id)))
}

pub async fn update_deal(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
    Form(form): Form<DealForm>,
) -> Result<Redirect, StatusCode> {
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

    // Default probability based on stage
    let probability = match form.stage.as_str() {
        "prospect" => 25,
        "negotiation" => 75,
        "closed_won" => 100,
        "closed_lost" => 0,
        _ => 50,
    };

    sqlx::query(
        r#"
        UPDATE deals SET
            customer_id = $2, contact_id = $3, title = $4, description = $5, value = $6,
            currency = $7, stage = $8, probability = $9, expected_close_date = $10, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&customer_id)
    .bind(&contact_id)
    .bind(&form.title)
    .bind(&form.description)
    .bind(&form.value)
    .bind(&form.currency)
    .bind(&form.stage)
    .bind(&probability)
    .bind(&form.expected_close_date)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/crm/deals/{}", id)))
}