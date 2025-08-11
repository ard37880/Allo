use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use askama::Template;
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use uuid::Uuid;
use chrono::{Utc, NaiveDate, NaiveDateTime};

use crate::{
    database::Database,
    models::{Customer, CustomerTemplate, Contact, Deal, Activity, CustomerDisplay, ContactDisplay, DealDisplay, ActivityDisplay},
    middleware::{get_current_user, CurrentUser},
    filters,
};

#[derive(Template)]
#[template(path = "crm/dashboard.html")]
struct CrmDashboardTemplate {
    customer_count: i64,
    deal_count: i64,
    total_deal_value: String,
    recent_activities: Vec<ActivityDisplay>,
    // Stage breakdown data
    prospect_deals: i64,
    prospect_value: String,
    negotiation_deals: i64,
    negotiation_value: String,
    closed_won_deals: i64,
    closed_won_value: String,
    closed_lost_deals: i64,
    closed_lost_value: String,
    // Performance metrics
    win_rate: i32,
    activities_this_month: i64,
    customer_change: i32,
    deals_change: i32,
    win_rate_change: i32,
    activities_change: i32,
}

#[derive(Template)]
#[template(path = "crm/customers.html")]
struct CustomersTemplate {
    customers: Vec<CustomerDisplay>,
}

#[derive(Template)]
#[template(path = "crm/customer_form.html")]
struct CustomerFormTemplate {
    customer: Option<CustomerTemplate>,
}

#[derive(Template)]
#[template(path = "crm/customer_detail.html")]
struct CustomerDetailTemplate {
    customer: CustomerDisplay,
    contacts: Vec<ContactDisplay>,
    deals: Vec<DealDisplay>,
    activities: Vec<ActivityDisplay>,
    current_user: CurrentUser,
}

#[derive(Template)]
#[template(path = "crm/deals.html")]
struct DealsTemplate {
    deals: Vec<DealDisplay>,
    current_user: CurrentUser,
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

#[derive(Template)]
#[template(path = "crm/activities.html")]
struct ActivitiesTemplate {
    activities: Vec<ActivityDisplay>,
    current_user: CurrentUser,
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
pub struct CustomerForm {
    company_name: String,
    industry: Option<String>,
    website: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address_line1: Option<String>,
    address_line2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
    status: String,
    notes: Option<String>,
}

#[derive(Deserialize)]
pub struct ContactForm {
    customer_id: Uuid,
    first_name: String,
    last_name: String,
    title: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    mobile: Option<String>,
    is_primary: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
pub struct DealQuery {
    customer_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct DealForm {
    customer_id: String,
    contact_id: Option<String>,
    title: String,
    description: Option<String>,
    value: Option<rust_decimal::Decimal>,
    currency: String,
    stage: String,
    expected_close_date: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub struct ActivityQuery {
    customer_id: Option<Uuid>,
    deal_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ActivityForm {
    customer_id: String,
    contact_id: Option<String>,
    deal_id: Option<String>,
    activity_type: String,
    subject: String,
    description: Option<String>,
    activity_date: String,
    duration_minutes: Option<i32>,
    completed: Option<String>,
}

// CRM Dashboard - FIXED VERSION WITH CORRECT PERFORMANCE METRICS
pub async fn crm_dashboard(State(db): State<Database>) -> Result<Html<String>, StatusCode> {
    let customer_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM customers")
        .fetch_one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Open deals are only prospect and negotiation (not closed)
    let open_deal_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE stage IN ('prospect', 'negotiation')"
    )
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_deal_value_raw = sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(
        "SELECT SUM(value) FROM deals WHERE stage NOT IN ('closed_lost')"
    )
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_deal_value = match total_deal_value_raw {
        Some(value) => format!("${}", value),
        None => "$0".to_string(),
    };

    // Calculate win rate
    let total_closed_deals = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE stage IN ('closed_won', 'closed_lost')"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let won_deals = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE stage = 'closed_won'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let win_rate = if total_closed_deals > 0 {
        ((won_deals as f64 / total_closed_deals as f64) * 100.0).round() as i32
    } else {
        0
    };

    // Calculate activities this month
    let activities_this_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM activities WHERE activity_date >= DATE_TRUNC('month', CURRENT_DATE)"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    // Calculate last month metrics for comparison
    let customers_last_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM customers WHERE created_at < DATE_TRUNC('month', CURRENT_DATE)"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let open_deals_last_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE created_at < DATE_TRUNC('month', CURRENT_DATE) AND stage IN ('prospect', 'negotiation')"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let activities_last_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM activities WHERE activity_date >= DATE_TRUNC('month', CURRENT_DATE - INTERVAL '1 month') AND activity_date < DATE_TRUNC('month', CURRENT_DATE)"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    // Calculate last month win rate
    let total_closed_deals_last_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE created_at < DATE_TRUNC('month', CURRENT_DATE) AND stage IN ('closed_won', 'closed_lost')"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let won_deals_last_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE created_at < DATE_TRUNC('month', CURRENT_DATE) AND stage = 'closed_won'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let win_rate_last_month = if total_closed_deals_last_month > 0 {
        ((won_deals_last_month as f64 / total_closed_deals_last_month as f64) * 100.0).round() as i32
    } else {
        0
    };

    // Calculate percentage changes
    let customer_change = if customers_last_month > 0 {
        (((customer_count - customers_last_month) as f64 / customers_last_month as f64) * 100.0).round() as i32
    } else if customer_count > 0 {
        100 // If we had 0 last month and have some now, that's 100% increase
    } else {
        0
    };

    let deals_change = if open_deals_last_month > 0 {
        (((open_deal_count - open_deals_last_month) as f64 / open_deals_last_month as f64) * 100.0).round() as i32
    } else if open_deal_count > 0 {
        100
    } else {
        0
    };

    let win_rate_change = win_rate - win_rate_last_month;

    let activities_change = if activities_last_month > 0 {
        (((activities_this_month - activities_last_month) as f64 / activities_last_month as f64) * 100.0).round() as i32
    } else if activities_this_month > 0 {
        100
    } else {
        0
    };

    // Get stage-specific data
    let prospect_deals = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE stage = 'prospect'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let prospect_value_raw = sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(
        "SELECT SUM(value) FROM deals WHERE stage = 'prospect'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(None);

    let prospect_value = match prospect_value_raw {
        Some(value) => format!("${}", value),
        None => "$0".to_string(),
    };

    let negotiation_deals = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE stage = 'negotiation'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let negotiation_value_raw = sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(
        "SELECT SUM(value) FROM deals WHERE stage = 'negotiation'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(None);

    let negotiation_value = match negotiation_value_raw {
        Some(value) => format!("${}", value),
        None => "$0".to_string(),
    };

    let closed_won_deals = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE stage = 'closed_won'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let closed_won_value_raw = sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(
        "SELECT SUM(value) FROM deals WHERE stage = 'closed_won'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(None);

    let closed_won_value = match closed_won_value_raw {
        Some(value) => format!("${}", value),
        None => "$0".to_string(),
    };

    let closed_lost_deals = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deals WHERE stage = 'closed_lost'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(0);

    let closed_lost_value_raw = sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(
        "SELECT SUM(value) FROM deals WHERE stage = 'closed_lost'"
    )
    .fetch_one(&db)
    .await
    .unwrap_or(None);

    let closed_lost_value = match closed_lost_value_raw {
        Some(value) => format!("${}", value),
        None => "$0".to_string(),
    };

    let recent_activities = sqlx::query_as::<_, Activity>(
        "SELECT * FROM activities ORDER BY activity_date DESC LIMIT 5"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(ActivityDisplay::from)
    .collect();

    let template = CrmDashboardTemplate {
        customer_count,
        deal_count: open_deal_count, // This should be open deals, not all deals
        total_deal_value,
        recent_activities,
        prospect_deals,
        prospect_value,
        negotiation_deals,
        negotiation_value,
        closed_won_deals,
        closed_won_value,
        closed_lost_deals,
        closed_lost_value,
        // Add new fields for performance metrics
        win_rate,
        activities_this_month,
        customer_change,
        deals_change,
        win_rate_change,
        activities_change,
    };
    
    Ok(Html(template.render().unwrap()))
}

// Customers List
pub async fn customers_list(State(db): State<Database>) -> Result<Html<String>, StatusCode> {
    let customers = sqlx::query_as::<_, Customer>(
        "SELECT * FROM customers ORDER BY created_at DESC"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(CustomerDisplay::from)
    .collect();

    let template = CustomersTemplate { customers };
    Ok(Html(template.render().unwrap()))
}

// Customer Form (New)
pub async fn customer_form() -> Html<String> {
    let template = CustomerFormTemplate {
        customer: None,
    };
    Html(template.render().unwrap())
}

// Customer Form (Edit)
pub async fn customer_edit_form(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let customer = sqlx::query_as::<_, Customer>(
        "SELECT * FROM customers WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    let template = CustomerFormTemplate {
        customer: Some(customer.into()),
    };
    Ok(Html(template.render().unwrap()))
}

// Create Customer
pub async fn create_customer(
    State(db): State<Database>,
    Form(form): Form<CustomerForm>,
) -> Result<Redirect, StatusCode> {
    let customer = sqlx::query_as::<_, Customer>(
        r#"
        INSERT INTO customers (
            company_name, industry, website, phone, email,
            address_line1, address_line2, city, state, postal_code,
            country, status, notes
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        RETURNING *
        "#,
    )
    .bind(&form.company_name)
    .bind(&form.industry)
    .bind(&form.website)
    .bind(&form.phone)
    .bind(&form.email)
    .bind(&form.address_line1)
    .bind(&form.address_line2)
    .bind(&form.city)
    .bind(&form.state)
    .bind(&form.postal_code)
    .bind(&form.country)
    .bind(&form.status)
    .bind(&form.notes)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/crm/customers/{}", customer.id)))
}

// Update Customer
pub async fn update_customer(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
    Form(form): Form<CustomerForm>,
) -> Result<Redirect, StatusCode> {
    let customer = sqlx::query_as::<_, Customer>(
        r#"
        UPDATE customers SET
            company_name = $2, industry = $3, website = $4, phone = $5, email = $6,
            address_line1 = $7, address_line2 = $8, city = $9, state = $10, postal_code = $11,
            country = $12, status = $13, notes = $14, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&form.company_name)
    .bind(&form.industry)
    .bind(&form.website)
    .bind(&form.phone)
    .bind(&form.email)
    .bind(&form.address_line1)
    .bind(&form.address_line2)
    .bind(&form.city)
    .bind(&form.state)
    .bind(&form.postal_code)
    .bind(&form.country)
    .bind(&form.status)
    .bind(&form.notes)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/crm/customers/{}", customer.id)))
}

// Customer Detail
pub async fn customer_detail(
    State(db): State<Database>,
    cookies: Cookies,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let customer = sqlx::query_as::<_, Customer>(
        "SELECT * FROM customers WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    let contacts = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts WHERE customer_id = $1 ORDER BY is_primary DESC, first_name"
    )
    .bind(id)
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(ContactDisplay::from)
    .collect();

    let deals = sqlx::query_as::<_, Deal>(
        "SELECT * FROM deals WHERE customer_id = $1 ORDER BY created_at DESC"
    )
    .bind(id)
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(DealDisplay::from)
    .collect();

    let activities = sqlx::query_as::<_, Activity>(
        "SELECT * FROM activities WHERE customer_id = $1 ORDER BY activity_date DESC LIMIT 10"
    )
    .bind(id)
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(ActivityDisplay::from)
    .collect();

    let template = CustomerDetailTemplate {
        customer: CustomerDisplay::from(customer),
        contacts,
        deals,
        activities,
        current_user,
    };
    
    Ok(Html(template.render().unwrap()))
}

// Create Contact
pub async fn create_contact(
    State(db): State<Database>,
    Form(form): Form<ContactForm>,
) -> Result<Redirect, StatusCode> {
    let is_primary = form.is_primary.is_some();
    
    if is_primary {
        sqlx::query("UPDATE contacts SET is_primary = false WHERE customer_id = $1")
            .bind(form.customer_id)
            .execute(&db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    sqlx::query(
        r#"
        INSERT INTO contacts (
            customer_id, first_name, last_name, title, email, phone, mobile, is_primary, notes
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(form.customer_id)
    .bind(&form.first_name)
    .bind(&form.last_name)
    .bind(&form.title)
    .bind(&form.email)
    .bind(&form.phone)
    .bind(&form.mobile)
    .bind(is_primary)
    .bind(&form.notes)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/crm/customers/{}", form.customer_id)))
}

// Delete Customer
pub async fn delete_customer(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    // First delete related contacts
    sqlx::query("DELETE FROM contacts WHERE customer_id = $1")
        .bind(id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Then delete related deals
    sqlx::query("DELETE FROM deals WHERE customer_id = $1")
        .bind(id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Then delete related activities
    sqlx::query("DELETE FROM activities WHERE customer_id = $1")
        .bind(id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Finally delete the customer
    sqlx::query("DELETE FROM customers WHERE id = $1")
        .bind(id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/crm/customers"))
}

// Delete Contact
pub async fn delete_contact(
    State(db): State<Database>,
    Path((customer_id, contact_id)): Path<(Uuid, Uuid)>,
) -> Result<Redirect, StatusCode> {
    sqlx::query("DELETE FROM contacts WHERE id = $1 AND customer_id = $2")
        .bind(contact_id)
        .bind(customer_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/crm/customers/{}", customer_id)))
}

// Deals functions
pub async fn deals_list(
    State(db): State<Database>,
    cookies: Cookies,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let deals = sqlx::query_as::<_, Deal>(
        "SELECT * FROM deals ORDER BY created_at DESC"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(DealDisplay::from)
    .collect();

    let template = DealsTemplate { deals, current_user };
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
    let user = get_current_user(cookies, &db).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let customer_id = Uuid::parse_str(&form.customer_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let contact_id = if let Some(contact_str) = form.contact_id {
        if contact_str.trim().is_empty() { None } 
        else { Some(Uuid::parse_str(&contact_str).map_err(|_| StatusCode::BAD_REQUEST)?) }
    } else { None };

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
    .map_err(|e| {
        eprintln!("Error creating deal: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Redirect::to(&format!("/crm/deals/{}", deal.id)))
}

pub async fn update_deal(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
    Form(form): Form<DealForm>,
 ) -> Result<Redirect, StatusCode> {
    let customer_id = Uuid::parse_str(&form.customer_id).map_err(|_| StatusCode::BAD_REQUEST)?;
 
    let contact_id = if let Some(contact_str) = form.contact_id {
        if contact_str.trim().is_empty() { None }
        else { Some(Uuid::parse_str(&contact_str).map_err(|_| StatusCode::BAD_REQUEST)?) }
    } else { None };
 
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
 

// Activities functions
pub async fn activities_list(
    State(db): State<Database>,
    cookies: Cookies,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let activities = sqlx::query_as::<_, Activity>(
       "SELECT * FROM activities ORDER BY activity_date DESC"
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(ActivityDisplay::from)
    .collect();

    let template = ActivitiesTemplate { activities, current_user };
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

// API function
#[derive(Serialize)]
pub struct ContactResponse {
   pub id: Uuid,
   pub first_name: String,
   pub last_name: String,
}

impl From<Contact> for ContactResponse {
   fn from(contact: Contact) -> Self {
       Self {
           id: contact.id,
           first_name: contact.first_name,
           last_name: contact.last_name,
       }
   }
}

pub async fn get_customer_contacts(
   State(db): State<Database>,
   Path(customer_id): Path<Uuid>,
) -> Result<axum::Json<Vec<ContactResponse>>, StatusCode> {
   let contacts = sqlx::query_as::<_, Contact>(
       "SELECT * FROM contacts WHERE customer_id = $1 ORDER BY first_name"
   )
   .bind(customer_id)
   .fetch_all(&db)
   .await
   .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
   .into_iter()
   .map(ContactResponse::from)
   .collect();

   Ok(axum::Json(contacts))
}

pub async fn delete_deal(
    State(db): State<Database>,
    cookies: Cookies,
    Path(deal_id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.permissions.contains(&"team:manage_roles".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query("DELETE FROM deals WHERE id = $1")
        .bind(deal_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/crm/deals"))
}

pub async fn delete_activity(
    State(db): State<Database>,
    cookies: Cookies,
    Path(activity_id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.permissions.contains(&"team:manage_roles".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query("DELETE FROM activities WHERE id = $1")
        .bind(activity_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/crm/activities"))
}

pub async fn activity_edit_form(
    State(db): State<Database>,
    Path(activity_id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let activity = sqlx::query_as::<_, Activity>("SELECT * FROM activities WHERE id = $1")
        .bind(activity_id)
        .fetch_one(&db)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let customers = sqlx::query_as::<_, Customer>("SELECT * FROM customers ORDER BY company_name")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let contacts = sqlx::query_as::<_, Contact>("SELECT * FROM contacts WHERE customer_id = $1 ORDER BY first_name")
        .bind(activity.customer_id)
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let deals = sqlx::query_as::<_, Deal>("SELECT * FROM deals WHERE customer_id = $1 ORDER BY title")
        .bind(activity.customer_id)
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = ActivityFormTemplate {
        activity: Some(activity),
        customers,
        contacts,
        deals,
        customer_id: None,
        deal_id: None,
    };

    Ok(Html(template.render().unwrap()))
}

pub async fn update_activity(
    State(db): State<Database>,
    Path(activity_id): Path<Uuid>,
    Form(form): Form<ActivityForm>,
) -> Result<Redirect, StatusCode> {
    let customer_id = Uuid::parse_str(&form.customer_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let contact_id = if let Some(contact_str) = form.contact_id {
        if contact_str.trim().is_empty() { None } 
        else { Some(Uuid::parse_str(&contact_str).map_err(|_| StatusCode::BAD_REQUEST)?) }
    } else { None };

    let deal_id = if let Some(deal_str) = form.deal_id {
        if deal_str.trim().is_empty() { None } 
        else { Some(Uuid::parse_str(&deal_str).map_err(|_| StatusCode::BAD_REQUEST)?) }
    } else { None };

    let activity_date = if form.activity_date.is_empty() {
        Utc::now()
    } else {
        NaiveDateTime::parse_from_str(&form.activity_date, "%Y-%m-%dT%H:%M")
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .and_utc()
    };

    let completed = form.completed.is_some();

    sqlx::query(
        r#"
        UPDATE activities SET
            customer_id = $2, contact_id = $3, deal_id = $4, activity_type = $5, subject = $6,
            description = $7, activity_date = $8, duration_minutes = $9, completed = $10
        WHERE id = $1
        "#,
    )
    .bind(activity_id)
    .bind(&customer_id)
    .bind(&contact_id)
    .bind(&deal_id)
    .bind(&form.activity_type)
    .bind(&form.subject)
    .bind(&form.description)
    .bind(&activity_date)
    .bind(&form.duration_minutes)
    .bind(completed)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/crm/activities"))
}
