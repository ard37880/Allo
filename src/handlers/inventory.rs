use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use askama::Template;
use tower_cookies::Cookies;
use uuid::Uuid;
use serde::Deserialize;
use rust_decimal::Decimal;
use std::str::FromStr;


use crate::{
    database::Database,
    models::{InventoryItem},
    middleware::{get_current_user, CurrentUser},
    filters,
};

#[derive(Template)]
#[template(path = "inventory/items.html")]
struct ItemsTemplate<'a> {
    items: Vec<InventoryItem>,
    current_user: &'a CurrentUser,
}

#[derive(Template)]
#[template(path = "inventory/item_form.html")]
struct ItemFormTemplate<'a> {
    item: Option<InventoryItem>,
    current_user: &'a CurrentUser,
}

// This struct now includes all the fields from your form
#[derive(Deserialize)]
pub struct ItemForm {
    item_name: String,
    sku: String,
    upc: Option<String>,
    item_type: String,
    category: Option<String>,
    brand: Option<String>,
    model: Option<String>,
    description: Option<String>,
    short_description: Option<String>,
    reorder_point: Option<String>,
    preferred_stock_level: Option<String>,
    lead_time: Option<String>,
    backorder_allowed: Option<String>, // HTML checkboxes send "on" or nothing
    purchase_price: Option<String>,
    selling_price: Option<String>,
    country_of_origin: Option<String>,
    hs_code: Option<String>,
}


// Handler to display the list of inventory items
pub async fn items_list(
    State(db): State<Database>,
    cookies: Cookies,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.permissions.contains(&"inventory:read".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let items = sqlx::query_as::<_, InventoryItem>("SELECT * FROM inventory_items ORDER BY item_name")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = ItemsTemplate { items, current_user: &current_user };
    Ok(Html(template.render().unwrap()))
}

// Handler to show the form for creating a new item
pub async fn item_form(
    cookies: Cookies,
    State(db): State<Database>,
) -> Result<Html<String>, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.permissions.contains(&"inventory:write".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let template = ItemFormTemplate { item: None, current_user: &current_user };
    Ok(Html(template.render().unwrap()))
}

// Handler to create a new inventory item
pub async fn create_item(
    State(db): State<Database>,
    cookies: Cookies,
    Form(form): Form<ItemForm>,
) -> Result<Redirect, StatusCode> {
    let current_user = get_current_user(cookies, &db).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !current_user.permissions.contains(&"inventory:write".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let backorder_allowed = form.backorder_allowed.is_some();
    
    // Helper closure to parse string to Option<Decimal>
    let parse_decimal = |s: Option<String>| -> Option<Decimal> {
        s.and_then(|val| Decimal::from_str(&val).ok())
    };

    // Helper closure to parse string to Option<i32>
    let parse_i32 = |s: Option<String>| -> Option<i32> {
        s.and_then(|val| val.parse::<i32>().ok())
    };

    sqlx::query(
        r#"
        INSERT INTO inventory_items (
            item_name, sku, upc, item_type, category, brand, model, description, short_description,
            reorder_point, preferred_stock_level, lead_time, backorder_allowed, purchase_price,
            selling_price, country_of_origin, hs_code, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        "#,
    )
    .bind(&form.item_name)
    .bind(&form.sku)
    .bind(&form.upc)
    .bind(&form.item_type)
    .bind(&form.category)
    .bind(&form.brand)
    .bind(&form.model)
    .bind(&form.description)
    .bind(&form.short_description)
    .bind(parse_i32(form.reorder_point).unwrap_or(0))
    .bind(parse_i32(form.preferred_stock_level).unwrap_or(0))
    .bind(parse_i32(form.lead_time))
    .bind(backorder_allowed)
    .bind(parse_decimal(form.purchase_price))
    .bind(parse_decimal(form.selling_price))
    .bind(&form.country_of_origin)
    .bind(&form.hs_code)
    .bind(current_user.id)
    .execute(&db)
    .await
    .map_err(|e| {
        eprintln!("Failed to create item: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Redirect::to("/inventory/items"))
}