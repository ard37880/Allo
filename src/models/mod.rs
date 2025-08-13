pub mod user;
pub mod crm;
pub mod rbac;
pub mod expense;
pub mod inventory; // Add this line

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
pub use expense::{Expense, ExpenseCategory, ExpenseDisplay};
pub use inventory::{ // Add these lines
    Warehouse, InventoryItem, StockLevel, StockMovement, Notification
};