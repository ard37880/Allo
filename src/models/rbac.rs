use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: sqlx::types::Json<Vec<String>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleDisplay {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub permission_count: usize,
}

impl From<Role> for RoleDisplay {
    fn from(role: Role) -> Self {
        let permissions = role.permissions.0.clone();
        Self {
            id: role.id,
            name: role.name,
            description: role.description.unwrap_or_default(),
            permission_count: permissions.len(),
            permissions,
            is_active: role.is_active,
            created_at: role.created_at,
            updated_at: role.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserRole {
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserWithRoles {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub is_active: bool,
    pub is_locked: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub locked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub roles: Vec<RoleDisplay>,
    pub permissions: Vec<String>,
}

// Fixed AuditLog struct without IpAddr to avoid sqlx compatibility issues
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub old_values: Option<sqlx::types::Json<serde_json::Value>>,
    pub new_values: Option<sqlx::types::Json<serde_json::Value>>,
    pub ip_address: Option<String>, // Changed from IpAddr to String
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogDisplay {
    pub id: Uuid,
    pub user_name: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub changes: String,
    pub ip_address: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRole {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Permission {
    pub key: String,
    pub name: String,
    pub description: String,
    pub category: String,
}

pub fn get_all_permissions() -> Vec<Permission> {
    vec![
        // Customer Management
        Permission {
            key: "customers:read".to_string(),
            name: "View Customers".to_string(),
            description: "View customer information and details".to_string(),
            category: "Customer Management".to_string(),
        },
        Permission {
            key: "customers:write".to_string(),
            name: "Manage Customers".to_string(),
            description: "Create and edit customer information".to_string(),
            category: "Customer Management".to_string(),
        },
        Permission {
            key: "customers:delete".to_string(),
            name: "Delete Customers".to_string(),
            description: "Delete customer records".to_string(),
            category: "Customer Management".to_string(),
        },
        
        // Inventory Management
        Permission {
            key: "inventory:read".to_string(),
            name: "View Inventory".to_string(),
            description: "View inventory items and stock levels".to_string(),
            category: "Inventory Management".to_string(),
        },
        Permission {
            key: "inventory:write".to_string(),
            name: "Manage Inventory".to_string(),
            description: "Create and edit inventory items".to_string(),
            category: "Inventory Management".to_string(),
        },
        Permission {
            key: "inventory:delete".to_string(),
            name: "Delete Inventory".to_string(),
            description: "Delete inventory items".to_string(),
            category: "Inventory Management".to_string(),
        },
        
        // Team Management
        Permission {
            key: "team:read".to_string(),
            name: "View Team".to_string(),
            description: "View team members and their information".to_string(),
            category: "Team Management".to_string(),
        },
        Permission {
            key: "team:write".to_string(),
            name: "Manage Team".to_string(),
            description: "Create and edit team member accounts".to_string(),
            category: "Team Management".to_string(),
        },
        Permission {
            key: "team:delete".to_string(),
            name: "Delete Team Members".to_string(),
            description: "Delete team member accounts".to_string(),
            category: "Team Management".to_string(),
        },
        Permission {
            key: "team:manage_roles".to_string(),
            name: "Manage Roles".to_string(),
            description: "Create, edit, and assign roles and permissions".to_string(),
            category: "Team Management".to_string(),
        },
        
        // Expense Tracking
        Permission {
            key: "expenses:read".to_string(),
            name: "View Expenses".to_string(),
            description: "View expense records and reports".to_string(),
            category: "Expense Tracking".to_string(),
        },
        Permission {
            key: "expenses:write".to_string(),
            name: "Manage Expenses".to_string(),
            description: "Create and edit expense records".to_string(),
            category: "Expense Tracking".to_string(),
        },
        Permission {
            key: "expenses:delete".to_string(),
            name: "Delete Expenses".to_string(),
            description: "Delete expense records".to_string(),
            category: "Expense Tracking".to_string(),
        },
        
        // Shipping Tracking
        Permission {
            key: "shipping:read".to_string(),
            name: "View Shipments".to_string(),
            description: "View shipment information and tracking".to_string(),
            category: "Shipping Tracking".to_string(),
        },
        Permission {
            key: "shipping:write".to_string(),
            name: "Manage Shipments".to_string(),
            description: "Create and edit shipment records".to_string(),
            category: "Shipping Tracking".to_string(),
        },
        Permission {
            key: "shipping:delete".to_string(),
            name: "Delete Shipments".to_string(),
            description: "Delete shipment records".to_string(),
            category: "Shipping Tracking".to_string(),
        },
        
        // API Access
        Permission {
            key: "api:access".to_string(),
            name: "API Access".to_string(),
            description: "Access API endpoints for integration".to_string(),
            category: "API Access".to_string(),
        },
        Permission {
            key: "api:admin".to_string(),
            name: "API Administration".to_string(),
            description: "Manage API keys and administrative functions".to_string(),
            category: "API Access".to_string(),
        },
    ]
}