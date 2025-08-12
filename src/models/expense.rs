use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ExpenseCategory {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Expense {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub amount: rust_decimal::Decimal,
    pub description: Option<String>,
    pub receipt_url: Option<String>,
    pub status: String,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub expense_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)] // MODIFIED: Added FromRow
pub struct ExpenseDisplay {
    pub id: Uuid,
    pub user_name: String,
    pub category_name: String,
    pub customer_name: Option<String>,
    pub amount: String,
    pub description: String,
    pub receipt_url: Option<String>,
    pub status: String,
    pub expense_date: String,
    pub created_at: DateTime<Utc>,
}