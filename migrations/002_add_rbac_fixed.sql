-- First, let's check what we have and fix the existing structure

-- Add missing columns to users table (only if they don't exist)
DO $$ 
BEGIN 
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'users' AND column_name = 'is_locked') THEN
        ALTER TABLE users ADD COLUMN is_locked BOOLEAN DEFAULT FALSE;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'users' AND column_name = 'last_login') THEN
        ALTER TABLE users ADD COLUMN last_login TIMESTAMP WITH TIME ZONE;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'users' AND column_name = 'locked_at') THEN
        ALTER TABLE users ADD COLUMN locked_at TIMESTAMP WITH TIME ZONE;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'users' AND column_name = 'locked_by') THEN
        ALTER TABLE users ADD COLUMN locked_by UUID REFERENCES users(id);
    END IF;
END $$;

-- Drop existing roles table if it has wrong structure
DROP TABLE IF EXISTS roles CASCADE;

-- Create roles table with correct structure
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id)
);

-- Drop and recreate user_roles table with correct structure
DROP TABLE IF EXISTS user_roles CASCADE;

CREATE TABLE user_roles (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    assigned_by UUID REFERENCES users(id),
    PRIMARY KEY (user_id, role_id)
);

-- Drop and recreate audit_logs table with correct column types for Rust compatibility
DROP TABLE IF EXISTS audit_logs CASCADE;

CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id UUID,
    old_values JSONB,
    new_values JSONB,
    ip_address TEXT, -- Changed from INET to TEXT for sqlx compatibility
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes (only if they don't exist)
CREATE INDEX IF NOT EXISTS idx_users_is_locked ON users(is_locked);
CREATE INDEX IF NOT EXISTS idx_users_last_login ON users(last_login);
CREATE INDEX IF NOT EXISTS idx_roles_name ON roles(name);
CREATE INDEX IF NOT EXISTS idx_roles_is_active ON roles(is_active);
CREATE INDEX IF NOT EXISTS idx_user_roles_user_id ON user_roles(user_id);
CREATE INDEX IF NOT EXISTS idx_user_roles_role_id ON user_roles(role_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_type ON audit_logs(resource_type);

-- Add update trigger to roles table
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_roles_updated_at BEFORE UPDATE ON roles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default roles with correct JSONB format
INSERT INTO roles (name, description, permissions) VALUES 
('Super Admin', 'Full system access with all permissions', 
 '["customers:read", "customers:write", "customers:delete", "inventory:read", "inventory:write", "inventory:delete", "team:read", "team:write", "team:delete", "team:manage_roles", "expenses:read", "expenses:write", "expenses:delete", "shipping:read", "shipping:write", "shipping:delete", "api:access", "api:admin"]'::jsonb),
('Manager', 'Management level access with most permissions', 
 '["customers:read", "customers:write", "inventory:read", "inventory:write", "team:read", "expenses:read", "expenses:write", "shipping:read", "shipping:write", "api:access"]'::jsonb),
('Sales Rep', 'Sales focused permissions for customer and deal management', 
 '["customers:read", "customers:write", "expenses:read", "api:access"]'::jsonb),
('Viewer', 'Read-only access to most resources', 
 '["customers:read", "inventory:read", "expenses:read", "shipping:read"]'::jsonb);

-- Assign Super Admin role to first user (if any users exist)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM users) THEN
        INSERT INTO user_roles (user_id, role_id, assigned_by)
        SELECT u.id, r.id, u.id 
        FROM users u, roles r 
        WHERE r.name = 'Super Admin' 
        AND u.id = (SELECT id FROM users ORDER BY created_at LIMIT 1);
    END IF;
END $$;

-- Show success message
SELECT 'RBAC system installed successfully!' as status;