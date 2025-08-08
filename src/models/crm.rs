use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Customer {
    pub id: Uuid,
    pub company_name: String,
    pub industry: Option<String>,
    pub website: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Template-friendly customer struct
#[derive(Debug, Serialize, Deserialize)]
pub struct CustomerTemplate {
    pub id: Uuid,
    pub company_name: String,
    pub industry: String,
    pub website: String,
    pub phone: String,
    pub email: String,
    pub address_line1: String,
    pub address_line2: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub status: String,
    pub notes: String,
}

impl From<Customer> for CustomerTemplate {
    fn from(customer: Customer) -> Self {
        Self {
            id: customer.id,
            company_name: customer.company_name,
            industry: customer.industry.unwrap_or_default(),
            website: customer.website.unwrap_or_default(),
            phone: customer.phone.unwrap_or_default(),
            email: customer.email.unwrap_or_default(),
            address_line1: customer.address_line1.unwrap_or_default(),
            address_line2: customer.address_line2.unwrap_or_default(),
            city: customer.city.unwrap_or_default(),
            state: customer.state.unwrap_or_default(),
            postal_code: customer.postal_code.unwrap_or_default(),
            country: customer.country.unwrap_or_else(|| "United States".to_string()),
            status: customer.status,
            notes: customer.notes.unwrap_or_default(),
        }
    }
}

// Template-friendly display version for listing and detail views
#[derive(Debug, Serialize, Deserialize)]
pub struct CustomerDisplay {
    pub id: Uuid,
    pub company_name: String,
    pub industry: String,
    pub website: String,
    pub phone: String,
    pub email: String,
    pub address_line1: String,
    pub address_line2: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub status: String,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Customer> for CustomerDisplay {
    fn from(customer: Customer) -> Self {
        Self {
            id: customer.id,
            company_name: customer.company_name,
            industry: customer.industry.unwrap_or_default(),
            website: customer.website.unwrap_or_default(),
            phone: customer.phone.unwrap_or_default(),
            email: customer.email.unwrap_or_default(),
            address_line1: customer.address_line1.unwrap_or_default(),
            address_line2: customer.address_line2.unwrap_or_default(),
            city: customer.city.unwrap_or_default(),
            state: customer.state.unwrap_or_default(),
            postal_code: customer.postal_code.unwrap_or_default(),
            country: customer.country.unwrap_or_else(|| "United States".to_string()),
            status: customer.status,
            notes: customer.notes.unwrap_or_default(),
            created_at: customer.created_at,
            updated_at: customer.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Contact {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub is_primary: bool,
    pub notes: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContactDisplay {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub title: String,
    pub email: String,
    pub phone: String,
    pub mobile: String,
    pub is_primary: bool,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Contact> for ContactDisplay {
    fn from(contact: Contact) -> Self {
        Self {
            id: contact.id,
            customer_id: contact.customer_id,
            first_name: contact.first_name,
            last_name: contact.last_name,
            title: contact.title.unwrap_or_default(),
            email: contact.email.unwrap_or_default(),
            phone: contact.phone.unwrap_or_default(),
            mobile: contact.mobile.unwrap_or_default(),
            is_primary: contact.is_primary,
            notes: contact.notes.unwrap_or_default(),
            created_at: contact.created_at,
            updated_at: contact.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Deal {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub contact_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub value: Option<rust_decimal::Decimal>,
    pub currency: String,
    pub stage: String,
    pub probability: i32,
    pub expected_close_date: Option<NaiveDate>,
    pub actual_close_date: Option<NaiveDate>,
    pub assigned_to: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DealDisplay {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub title: String,
    pub description: String,
    pub value: String,
    pub currency: String,
    pub stage: String,
    pub probability: i32,
    pub expected_close_date: String,
    pub actual_close_date: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Deal> for DealDisplay {
    fn from(deal: Deal) -> Self {
        Self {
            id: deal.id,
            customer_id: deal.customer_id,
            title: deal.title,
            description: deal.description.unwrap_or_default(),
            value: deal.value.map(|v| format!("{}", v)).unwrap_or_default(),
            currency: deal.currency,
            stage: deal.stage,
            probability: deal.probability,
            expected_close_date: deal.expected_close_date.map(|d| d.to_string()).unwrap_or_default(),
            actual_close_date: deal.actual_close_date.map(|d| d.to_string()).unwrap_or_default(),
            created_at: deal.created_at,
            updated_at: deal.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Activity {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub contact_id: Option<Uuid>,
    pub deal_id: Option<Uuid>,
    pub activity_type: String,
    pub subject: String,
    pub description: Option<String>,
    pub activity_date: DateTime<Utc>,
    pub duration_minutes: Option<i32>,
    pub completed: bool,
    pub assigned_to: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityDisplay {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub activity_type: String,
    pub subject: String,
    pub description: String,
    pub activity_date: String,
    pub duration_minutes: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Activity> for ActivityDisplay {
    fn from(activity: Activity) -> Self {
        Self {
            id: activity.id,
            customer_id: activity.customer_id,
            activity_type: activity.activity_type,
            subject: activity.subject,
            description: activity.description.unwrap_or_default(),
            activity_date: activity.activity_date.format("%B %d, %Y at %I:%M %p").to_string(),
            duration_minutes: activity.duration_minutes.map(|d| d.to_string()).unwrap_or_default(),
            completed: activity.completed,
            created_at: activity.created_at,
            updated_at: activity.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCustomer {
    pub company_name: String,
    pub industry: Option<String>,
    pub website: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub status: String,
    pub notes: Option<String>,
}