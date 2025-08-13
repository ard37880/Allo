use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Warehouse {
    pub id: Uuid,
    pub name: String,
    pub location: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct InventoryItem {
    pub id: Uuid,
    pub item_name: String,
    pub sku: String,
    pub upc: Option<String>,
    pub item_type: String,
    pub category: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub image_url: Option<String>,
    pub reorder_point: i32,
    pub preferred_stock_level: i32,
    pub lead_time: Option<i32>,
    pub backorder_allowed: bool,
    pub preferred_supplier_id: Option<Uuid>,
    pub purchase_price: Option<rust_decimal::Decimal>,
    pub selling_price: Option<rust_decimal::Decimal>,
    pub tax_category: Option<String>,
    pub cost_price: Option<rust_decimal::Decimal>,
    pub landed_cost: Option<rust_decimal::Decimal>,
    pub average_cost: Option<rust_decimal::Decimal>,
    pub gross_margin: Option<rust_decimal::Decimal>,
    pub currency: String,
    pub country_of_origin: Option<String>,
    pub hs_code: Option<String>,
    pub lifecycle_stage: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct StockLevel {
    pub item_id: Uuid,
    pub warehouse_id: Uuid,
    pub quantity_on_hand: i32,
    pub quantity_committed: i32,
    pub quantity_available: i32,
    pub aisle: Option<String>,
    pub bin: Option<String>,
    pub lot_number: Option<String>,
    pub serial_number: Option<String>,
    pub expiry_date: Option<NaiveDate>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct StockMovement {
    pub id: Uuid,
    pub item_id: Uuid,
    pub from_warehouse_id: Option<Uuid>,
    pub to_warehouse_id: Option<Uuid>,
    pub quantity: i32,
    pub movement_type: String,
    pub reason: Option<String>,
    pub reference_id: Option<String>,
    pub moved_by: Option<Uuid>,
    pub moved_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub link_url: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}