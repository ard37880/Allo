-- First, let's define the specific permissions required for stock management.
-- We will add these permissions to the Super Admin role to ensure full access.
UPDATE roles
SET permissions = permissions || '["items:read", "items:write", "items:delete", "warehouses:read", "warehouses:write", "warehouses:delete", "stock_movements:read", "notifications:read"]'::jsonb
WHERE name = 'Super Admin';

-- Create the "Inventory" role with permissions to view/edit items and view stock movements.
INSERT INTO roles (name, description, permissions) VALUES
('Inventory', 'Can view and edit items and track stock movements.',
'["items:read", "items:write", "stock_movements:read"]'::jsonb)
ON CONFLICT (name) DO UPDATE SET
description = EXCLUDED.description,
permissions = EXCLUDED.permissions;

-- Create the "Inventory Manager" role with permissions to delete items, manage warehouses, and receive alerts.
INSERT INTO roles (name, description, permissions) VALUES
('Inventory Manager', 'Full control over inventory, warehouses, and low-stock alerts.',
'["items:read", "items:write", "items:delete", "warehouses:read", "warehouses:write", "warehouses:delete", "stock_movements:read", "notifications:read"]'::jsonb)
ON CONFLICT (name) DO UPDATE SET
description = EXCLUDED.description,
permissions = EXCLUDED.permissions;

SELECT 'Inventory roles and permissions seeded successfully!' as status;
