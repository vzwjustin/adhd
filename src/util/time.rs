use chrono::{DateTime, Utc};

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

pub fn format_relative(dt: DateTime<Utc>) -> String {
    let duration = Utc::now().signed_duration_since(dt);
    let secs = duration.num_seconds();
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        let mins = secs / 60;
        format!("{mins}m ago")
    } else if secs < 86400 {
        let hours = secs / 3600;
        format!("{hours}h ago")
    } else {
        let days = secs / 86400;
        format!("{days}d ago")
    }
}

pub fn format_duration_short(secs: i64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
