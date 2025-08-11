use askama::Result;

// Custom filter to check if a Vec<String> contains a specific string.
// This allows us to use `|contains("value")` in the templates.
#[allow(clippy::unnecessary_wraps)]
pub fn contains(s: &Vec<String>, v: &str) -> Result<bool> {
    Ok(s.contains(&v.to_string()))
}