use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    database::Database,
    models::Contact,
};

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
) -> Result<Json<Vec<ContactResponse>>, StatusCode> {
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

    Ok(Json(contacts))
}