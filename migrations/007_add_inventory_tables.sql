-- Create warehouses table
CREATE TABLE IF NOT EXISTS warehouses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    location TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    created_by UUID REFERENCES users(id)
);

-- Create inventory_items table
CREATE TABLE IF NOT EXISTS inventory_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_name VARCHAR(255) NOT NULL,
    sku VARCHAR(100) UNIQUE NOT NULL,
    upc VARCHAR(100),
    item_type VARCHAR(50) NOT NULL,
    category VARCHAR(100),
    brand VARCHAR(100),
    model VARCHAR(100),
    description TEXT,
    short_description VARCHAR(255),
    image_url TEXT,
    reorder_point INTEGER DEFAULT 0,
    preferred_stock_level INTEGER DEFAULT 0,
    lead_time INTEGER, -- in days
    backorder_allowed BOOLEAN DEFAULT FALSE,
    preferred_supplier_id UUID, -- Can be linked to a future suppliers table
    purchase_price DECIMAL(15, 2),
    selling_price DECIMAL(15, 2),
    tax_category VARCHAR(50),
    cost_price DECIMAL(15, 2),
    landed_cost DECIMAL(15, 2),
    average_cost DECIMAL(15, 2),
    gross_margin DECIMAL(5, 2),
    currency VARCHAR(3) DEFAULT 'USD',
    country_of_origin VARCHAR(100),
    hs_code VARCHAR(100),
    lifecycle_stage VARCHAR(50) DEFAULT 'active',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    created_by UUID REFERENCES users(id)
);

-- Create stock_levels table (junction table for items and warehouses)
CREATE TABLE IF NOT EXISTS stock_levels (
    item_id UUID NOT NULL REFERENCES inventory_items(id) ON DELETE CASCADE,
    warehouse_id UUID NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    quantity_on_hand INTEGER NOT NULL DEFAULT 0,
    quantity_committed INTEGER NOT NULL DEFAULT 0,
    quantity_available INTEGER NOT NULL DEFAULT 0,
    aisle VARCHAR(50),
    bin VARCHAR(50),
    lot_number VARCHAR(100),
    serial_number VARCHAR(255),
    expiry_date DATE,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (item_id, warehouse_id)
);

-- Create stock_movements table
CREATE TABLE IF NOT EXISTS stock_movements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id UUID NOT NULL REFERENCES inventory_items(id),
    from_warehouse_id UUID REFERENCES warehouses(id),
    to_warehouse_id UUID REFERENCES warehouses(id),
    quantity INTEGER NOT NULL,
    movement_type VARCHAR(50) NOT NULL, -- e.g., 'purchase', 'sale', 'transfer', 'adjustment'
    reason TEXT,
    reference_id VARCHAR(255), -- e.g., PO number, SO number
    moved_by UUID REFERENCES users(id),
    moved_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create notifications table
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    message TEXT NOT NULL,
    link_url VARCHAR(255),
    is_read BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes
CREATE INDEX idx_inventory_items_sku ON inventory_items(sku);
CREATE INDEX idx_stock_levels_item_id ON stock_levels(item_id);
CREATE INDEX idx_stock_levels_warehouse_id ON stock_levels(warehouse_id);
CREATE INDEX idx_stock_movements_item_id ON stock_movements(item_id);
CREATE INDEX idx_notifications_user_id ON notifications(user_id);

-- Apply update triggers
CREATE TRIGGER update_warehouses_updated_at BEFORE UPDATE ON warehouses
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_inventory_items_updated_at BEFORE UPDATE ON inventory_items
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_stock_levels_updated_at BEFORE UPDATE ON stock_levels
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add inventory permissions to roles
UPDATE roles
SET permissions = permissions || '["inventory:read", "inventory:write"]'::jsonb
WHERE name = 'Super Admin';

-- Create Inventory Manager role
INSERT INTO roles (name, description, permissions)
VALUES ('Inventory Manager', 'Full access to inventory, warehouses, and stock movements',
'["inventory:read", "inventory:write", "inventory:delete", "inventory:manage_warehouses"]'::jsonb);

-- Create Inventory role
INSERT INTO roles (name, description, permissions)
VALUES ('Inventory', 'View and edit inventory items and view stock movements',
'["inventory:read", "inventory:write"]'::jsonb);

SELECT 'Inventory tables created and roles updated successfully!' as status;