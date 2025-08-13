-- Add new columns to the items table to support detailed item tracking
ALTER TABLE items
    ADD COLUMN item_type VARCHAR(50),
    ADD COLUMN upc VARCHAR(255),
    ADD COLUMN brand VARCHAR(100),
    ADD COLUMN model VARCHAR(100),
    ADD COLUMN short_description VARCHAR(255),
    ADD COLUMN image_urls TEXT[],
    ADD COLUMN quantity_on_hand INTEGER DEFAULT 0,
    ADD COLUMN quantity_committed INTEGER DEFAULT 0,
    ADD COLUMN preferred_stock_level INTEGER,
    ADD COLUMN backorder_allowed BOOLEAN DEFAULT FALSE,
    ADD COLUMN lead_time INTEGER,
    ADD COLUMN lot_batch_numbers TEXT[],
    ADD COLUMN serial_numbers TEXT[],
    ADD COLUMN expiry_date DATE,
    ADD COLUMN preferred_supplier VARCHAR(255),
    ADD COLUMN supplier_sku VARCHAR(100),
    ADD COLUMN purchase_price DECIMAL(15, 2),
    ADD COLUMN minimum_order_quantity INTEGER,
    ADD COLUMN last_purchase_date DATE,
    ADD COLUMN tax_category VARCHAR(50),
    ADD COLUMN sales_channels TEXT[],
    ADD COLUMN warranty_info TEXT,
    ADD COLUMN landed_cost DECIMAL(15, 2),
    ADD COLUMN average_cost DECIMAL(15, 2),
    ADD COLUMN currency VARCHAR(3) DEFAULT 'USD',
    ADD COLUMN material_safety_data_sheet_url TEXT,
    ADD COLUMN hazard_classification VARCHAR(100),
    ADD COLUMN country_of_origin VARCHAR(100),
    ADD COLUMN customs_tariff_code VARCHAR(100),
    ADD COLUMN lifecycle_stage VARCHAR(50),
    ADD COLUMN priority VARCHAR(50);

-- Add tracking categories as a JSONB field for flexibility
ALTER TABLE items
    ADD COLUMN tracking_categories JSONB;

SELECT 'Items table expanded successfully!' as status;
