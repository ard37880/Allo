use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use axum_extra::extract::Multipart;
use askama::Template;
use serde::Deserialize;
use tower_cookies::Cookies;
use uuid::Uuid;
use chrono::{NaiveDate, Utc};
use std::path::PathBuf;
use tokio::fs;

use crate::{
    database::Database,
    models::{Expense, ExpenseCategory, ExpenseDisplay, Customer, User},
    middleware::{get_current_user, CurrentUser},
};

// MODIFIED: This struct now accepts dates as optional strings.
// This is the key change to prevent the deserialization error when
// date fields are submitted empty from the form.
#[derive(Deserialize)]
pub struct ExpenseFilters {
    #[serde(default)]
    user_id: String,
    #[serde(default)]
    category_id: String,
    #[serde(default)]
    customer_id: String,
    date_from: Option<String>,
    date_to: Option<String>,
}

#[derive(Template)]
#[template(path = "expenses/expenses.html")]
struct ExpensesTemplate {
    expenses: Vec<ExpenseDisplay>,
    current_user: CurrentUser,
    users: Vec<User>,
    categories: Vec<ExpenseCategory>,
    customers: Vec<Customer>,
    selected_user: Option<Uuid>,
    selected_category: Option<Uuid>,
    selected_customer: Option<Uuid>,
    selected_date_from: String,
    selected_date_to: String,
}

#[derive(Template)]
#[template(path = "expenses/expense_form.html")]
struct ExpenseFormTemplate {
    expense: Option<Expense>,
    categories: Vec<ExpenseCategory>,
    customers: Vec<Customer>,
}

// MODIFIED: The logic inside this function is updated to handle the string-to-date parsing.
pub async fn expenses_list(
    State(db): State<Database>,
    cookies: Cookies,
    Query(filters): Query<ExpenseFilters>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = Uuid::parse_str(&filters.user_id).ok();
    let category_id = Uuid::parse_str(&filters.category_id).ok();
    let customer_id = Uuid::parse_str(&filters.customer_id).ok();

    // Manually parse the date strings into Option<NaiveDate>.
    // This checks if the string is empty before attempting to parse it.
    let date_from = filters.date_from.as_deref()
        .and_then(|s| if s.is_empty() { None } else { NaiveDate::parse_from_str(s, "%Y-%m-%d").ok() });
    let date_to = filters.date_to.as_deref()
        .and_then(|s| if s.is_empty() { None } else { NaiveDate::parse_from_str(s, "%Y-%m-%d").ok() });


    let users = sqlx::query_as("SELECT * FROM users ORDER BY first_name, last_name").fetch_all(&db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let categories = sqlx::query_as("SELECT * FROM expense_categories ORDER BY name").fetch_all(&db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let customers = sqlx::query_as("SELECT * FROM customers ORDER BY company_name").fetch_all(&db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut query_builder = sqlx::QueryBuilder::new(
        r#"
        SELECT
            e.id,
            CONCAT(u.first_name, ' ', u.last_name) as user_name,
            ec.name as category_name,
            c.company_name as customer_name,
            e.amount::text,
            COALESCE(e.description, '') as description,
            e.receipt_url,
            e.status,
            e.expense_date::text,
            e.created_at
        FROM expenses e
        JOIN users u ON e.user_id = u.id
        JOIN expense_categories ec ON e.category_id = ec.id
        LEFT JOIN customers c ON e.customer_id = c.id
        "#,
    );

    let mut conditions = Vec::new();

    if let Some(id) = user_id {
        conditions.push(format!("e.user_id = '{}'", id));
    }
    if let Some(id) = category_id {
        conditions.push(format!("e.category_id = '{}'", id));
    }
    if let Some(id) = customer_id {
        conditions.push(format!("e.customer_id = '{}'", id));
    }
    // Now we use the parsed date_from and date_to variables
    if let Some(date) = date_from {
        conditions.push(format!("e.expense_date >= '{}'", date));
    }
    if let Some(date) = date_to {
        conditions.push(format!("e.expense_date <= '{}'", date));
    }

    if !conditions.is_empty() {
        query_builder.push(" WHERE ");
        query_builder.push(conditions.join(" AND "));
    }

    query_builder.push(" ORDER BY e.expense_date DESC");

    let expenses = query_builder.build_query_as::<ExpenseDisplay>()
        .fetch_all(&db)
        .await
        .map_err(|e| {
            eprintln!("Failed to fetch expenses: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let template = ExpensesTemplate {
        expenses,
        current_user,
        users,
        categories,
        customers,
        selected_user: user_id,
        selected_category: category_id,
        selected_customer: customer_id,
        // Pass the original string values back to the template.
        // This ensures the form fields show what the user last entered.
        selected_date_from: filters.date_from.unwrap_or_default(),
        selected_date_to: filters.date_to.unwrap_or_default(),
    };

    Ok(Html(template.render().unwrap()))
}

pub async fn approve_expense(
    State(db): State<Database>,
    cookies: Cookies,
    Path(expense_id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await.ok_or(StatusCode::UNAUTHORIZED)?;
    if !current_user.has_expense_approval {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query(
        "UPDATE expenses SET status = 'approved', approved_by = $1, approved_at = NOW() WHERE id = $2"
    )
    .bind(current_user.id)
    .bind(expense_id)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/expenses"))
}

pub async fn deny_expense(
    State(db): State<Database>,
    cookies: Cookies,
    Path(expense_id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await.ok_or(StatusCode::UNAUTHORIZED)?;
    if !current_user.has_expense_approval {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query(
        "UPDATE expenses SET status = 'denied', approved_by = $1, approved_at = NOW() WHERE id = $2"
    )
    .bind(current_user.id)
    .bind(expense_id)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/expenses"))
}

pub async fn expense_form(
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let categories = sqlx::query_as("SELECT * FROM expense_categories WHERE is_active = true ORDER BY name")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let customers = sqlx::query_as("SELECT * FROM customers ORDER BY company_name")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = ExpenseFormTemplate {
        expense: None,
        categories,
        customers,
    };
    Ok(Html(template.render().unwrap()))
}

pub async fn expense_edit_form(
    State(db): State<Database>,
    Path(expense_id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let expense = sqlx::query_as("SELECT * FROM expenses WHERE id = $1")
        .bind(expense_id)
        .fetch_optional(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let categories = sqlx::query_as("SELECT * FROM expense_categories WHERE is_active = true ORDER BY name")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let customers = sqlx::query_as("SELECT * FROM customers ORDER BY company_name")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = ExpenseFormTemplate {
        expense: Some(expense),
        categories,
        customers,
    };

    Ok(Html(template.render().unwrap()))
}

pub async fn create_expense(
    State(db): State<Database>,
    cookies: Cookies,
    multipart: Multipart,
) -> Result<Redirect, StatusCode> {
    let user = get_current_user(cookies, &db).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let (form_data, receipt_data) = parse_expense_multipart(multipart).await?;

    let (category_id, amount, expense_date) = match (
        form_data.category_id,
        form_data.amount,
        form_data.expense_date,
    ) {
        (Some(c), Some(a), Some(d)) => (c, a, d),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    let receipt_url = save_receipt(receipt_data).await?;

    sqlx::query(
        "INSERT INTO expenses (user_id, category_id, customer_id, amount, description, expense_date, receipt_url) VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(user.id)
    .bind(category_id)
    .bind(form_data.customer_id)
    .bind(amount)
    .bind(form_data.description)
    .bind(expense_date)
    .bind(receipt_url)
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/expenses"))
}

pub async fn update_expense(
    State(db): State<Database>,
    Path(expense_id): Path<Uuid>,
    multipart: Multipart,
) -> Result<Redirect, StatusCode> {
    let (form_data, receipt_data) = parse_expense_multipart(multipart).await?;
    
    let (category_id, amount, expense_date) = match (
        form_data.category_id,
        form_data.amount,
        form_data.expense_date,
    ) {
        (Some(c), Some(a), Some(d)) => (c, a, d),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    let receipt_url = save_receipt(receipt_data).await?;

    if receipt_url.is_some() {
        sqlx::query(
            "UPDATE expenses SET category_id = $1, customer_id = $2, amount = $3, description = $4, expense_date = $5, receipt_url = $6, updated_at = NOW() WHERE id = $7"
        )
        .bind(category_id)
        .bind(form_data.customer_id)
        .bind(amount)
        .bind(form_data.description)
        .bind(expense_date)
        .bind(receipt_url)
        .bind(expense_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        sqlx::query(
            "UPDATE expenses SET category_id = $1, customer_id = $2, amount = $3, description = $4, expense_date = $5, updated_at = NOW() WHERE id = $6"
        )
        .bind(category_id)
        .bind(form_data.customer_id)
        .bind(amount)
        .bind(form_data.description)
        .bind(expense_date)
        .bind(expense_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Redirect::to("/expenses"))
}

pub async fn delete_expense(
    State(db): State<Database>,
    Path(expense_id): Path<Uuid>,
) -> Result<Redirect, StatusCode> {
    sqlx::query("DELETE FROM expenses WHERE id = $1")
        .bind(expense_id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/expenses"))
}

struct ExpenseFormData {
    category_id: Option<Uuid>,
    customer_id: Option<Uuid>,
    amount: Option<rust_decimal::Decimal>,
    description: Option<String>,
    expense_date: Option<NaiveDate>,
}

struct ReceiptData {
    filename: Option<String>,
    data: axum::body::Bytes,
}

async fn parse_expense_multipart(mut multipart: Multipart) -> Result<(ExpenseFormData, Option<ReceiptData>), StatusCode> {
    let mut form_data = ExpenseFormData {
        category_id: None,
        customer_id: None,
        amount: None,
        description: None,
        expense_date: None,
    };
    let mut receipt_data = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = match field.name() {
            Some(name) => name.to_string(),
            None => continue,
        };

        if name == "receipt" {
            let filename = field.file_name().map(|s| s.to_string());
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            if filename.is_some() && !data.is_empty() {
                receipt_data = Some(ReceiptData { filename, data });
            }
        } else {
            let text_value = String::from_utf8(field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_vec())
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            if !text_value.is_empty() {
                match name.as_str() {
                    "category_id" => form_data.category_id = Uuid::parse_str(&text_value).ok(),
                    "customer_id" => form_data.customer_id = Uuid::parse_str(&text_value).ok(),
                    "amount" => form_data.amount = rust_decimal::Decimal::from_str_radix(&text_value, 10).ok(),
                    "description" => form_data.description = Some(text_value),
                    "expense_date" => form_data.expense_date = NaiveDate::parse_from_str(&text_value, "%Y-%m-%d").ok(),
                    _ => (),
                }
            }
        }
    }
    Ok((form_data, receipt_data))
}

async fn save_receipt(receipt_data: Option<ReceiptData>) -> Result<Option<String>, StatusCode> {
    if let Some(receipt) = receipt_data {
        if let Some(fname) = receipt.filename {
            let receipts_dir = PathBuf::from("static/receipts");
            if !receipts_dir.exists() {
                fs::create_dir_all(&receipts_dir).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            let extension = PathBuf::from(&fname).extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if ["png", "jpg", "jpeg"].contains(&extension.as_str()) {
                let new_file_name = format!("{}.{}", Uuid::new_v4(), extension);
                let file_path = receipts_dir.join(&new_file_name);
                fs::write(&file_path, &receipt.data).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(Some(format!("/static/receipts/{}", new_file_name)));
            }
        }
    }
    Ok(None)
}