use askama::Result;

pub fn display_optional<T: std::fmt::Display>(opt: &Option<T>) -> Result<String> {
    Ok(match opt {
        Some(val) => val.to_string(),
        None => String::new(),
    })
}

pub fn is_empty(s: &str) -> Result<bool> {
    Ok(s.is_empty())
}

// Helper for templates to check if a vector contains a specific item
pub fn contains<T: PartialEq>(vec: &[T], item: &T) -> Result<bool> {
    Ok(vec.contains(item))
}

// String replacement filter
pub fn replace(s: &str, from: &str, to: &str) -> Result<String> {
    Ok(s.replace(from, to))
}

// Title case filter
pub fn title(s: &str) -> Result<String> {
    Ok(s.chars()
        .enumerate()
        .map(|(i, c)| {
            if i == 0 || s.chars().nth(i - 1).unwrap_or(' ') == ' ' {
                c.to_uppercase().collect::<String>()
            } else {
                c.to_lowercase().collect::<String>()
            }
        })
        .collect())
}

// Date formatting filter
pub fn date(date: &chrono::DateTime<chrono::Utc>, format: &str) -> Result<String> {
    Ok(date.format(format).to_string())
}