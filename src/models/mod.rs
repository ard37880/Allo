pub mod user;
pub mod crm;
pub mod rbac;

// Re-export only the types we actually use
pub use user::{User, CreateUser};
pub use crm::{
    Customer, CustomerTemplate, CustomerDisplay,
    Contact, ContactDisplay,
    Deal, DealDisplay,
    Activity, ActivityDisplay
};
pub use rbac::{
    Role, RoleDisplay, UserWithRoles,
    Permission, get_all_permissions
};