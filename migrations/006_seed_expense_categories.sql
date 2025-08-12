-- Insert default expense categories
INSERT INTO expense_categories (name, description, is_active) VALUES
('Travel', 'Expenses related to business travel, including flights, hotels, and transportation.', TRUE),
('Food', 'Expenses for meals, including business lunches and per diems.', TRUE),
('Operating', 'General operating expenses like office supplies, utilities, and rent.', TRUE),
('Marketing', 'Costs associated with marketing and advertising efforts.', TRUE),
('Professional', 'Fees for professional services, subscriptions, and training.', TRUE),
('Costs of Goods Sold', 'Direct costs attributable to the production of the goods sold by a company.', TRUE),
('Financial', 'Bank fees, interest charges, and other financial costs.', TRUE),
('Miscellaneous', 'Other uncategorized business expenses.', TRUE)
ON CONFLICT (name) DO NOTHING; -- This prevents errors if you run the migration more than once

SELECT 'Default expense categories seeded successfully!' as status;