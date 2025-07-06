-- Add timestamps to documents table
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP;

-- Create index on created_at for efficient sorting
CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at);

-- Create a trigger to automatically update updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER IF NOT EXISTS update_documents_updated_at 
    BEFORE UPDATE ON documents 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();
