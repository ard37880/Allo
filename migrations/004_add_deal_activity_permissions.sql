-- Add Deal and Activity deletion permissions to Super Admin role
UPDATE roles 
SET permissions = (
    SELECT jsonb_agg(DISTINCT elem)
    FROM (
        SELECT jsonb_array_elements_text(permissions) as elem
        FROM roles WHERE id = roles.id
        UNION ALL
        SELECT unnest(ARRAY['deals:read', 'deals:write', 'deals:delete', 'activities:read', 'activities:write', 'activities:delete'])
    ) combined
)
WHERE name = 'Super Admin';

-- Also add basic deal and activity permissions to Manager role
UPDATE roles 
SET permissions = (
    SELECT jsonb_agg(DISTINCT elem)
    FROM (
        SELECT jsonb_array_elements_text(permissions) as elem
        FROM roles WHERE id = roles.id
        UNION ALL
        SELECT unnest(ARRAY['deals:read', 'deals:write', 'activities:read', 'activities:write'])
    ) combined
)
WHERE name = 'Manager';

-- Also add basic deal and activity permissions to Sales Rep role
UPDATE roles 
SET permissions = (
    SELECT jsonb_agg(DISTINCT elem)
    FROM (
        SELECT jsonb_array_elements_text(permissions) as elem
        FROM roles WHERE id = roles.id
        UNION ALL
        SELECT unnest(ARRAY['deals:read', 'deals:write', 'activities:read', 'activities:write'])
    ) combined
)
WHERE name = 'Sales Rep';

-- Add read-only deal and activity permissions to Viewer role
UPDATE roles 
SET permissions = (
    SELECT jsonb_agg(DISTINCT elem)
    FROM (
        SELECT jsonb_array_elements_text(permissions) as elem
        FROM roles WHERE id = roles.id
        UNION ALL
        SELECT unnest(ARRAY['deals:read', 'activities:read'])
    ) combined
)
WHERE name = 'Viewer';

SELECT 'Deal and Activity permissions added successfully!' as status;