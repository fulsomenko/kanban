use chrono::{DateTime, NaiveDate, Utc};

pub fn parse_datetime_input(input: &str) -> Result<DateTime<Utc>, String> {
    let trimmed = input.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        // Enforce strict ISO 8601 zero-padding: chrono's %m/%d accept 1-2 digits,
        // so re-format and require an exact match to reject inputs like "2024-1-5".
        if date.format("%Y-%m-%d").to_string() == trimmed {
            let naive = date
                .and_hms_opt(0, 0, 0)
                .expect("midnight is always a valid time");
            return Ok(naive.and_utc());
        }
    }
    Err(format!(
        "Invalid date '{input}'. Supported formats: YYYY-MM-DD or RFC 3339 (e.g. 2024-01-15 or 2024-01-15T14:30:00Z)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_parse_yyyy_mm_dd_returns_midnight_utc() {
        let dt = parse_datetime_input("2024-01-15").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap());
    }

    #[test]
    fn test_parse_full_rfc3339_z_suffix_preserved() {
        let dt = parse_datetime_input("2024-01-15T14:30:00Z").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap());
    }

    #[test]
    fn test_parse_full_rfc3339_with_offset_normalized_to_utc() {
        let dt = parse_datetime_input("2024-01-15T16:30:00+02:00").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap());
    }

    #[test]
    fn test_parse_leading_and_trailing_whitespace_tolerated() {
        let dt = parse_datetime_input("  2024-01-15  ").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap());
    }

    #[test]
    fn test_parse_iso8601_non_zero_padded_date_rejected() {
        let err = parse_datetime_input("2024-1-5").unwrap_err();
        assert!(
            err.contains("2024-1-5"),
            "error should quote the offending input, got: {err}"
        );
    }

    #[test]
    fn test_parse_garbage_input_returns_descriptive_error() {
        let err = parse_datetime_input("yesterday").unwrap_err();
        assert!(
            err.contains("yesterday"),
            "error should quote the offending input, got: {err}"
        );
        assert!(
            err.contains("YYYY-MM-DD"),
            "error should mention the supported format, got: {err}"
        );
    }

    #[test]
    fn test_parse_empty_string_returns_error() {
        assert!(parse_datetime_input("").is_err());
    }

    #[test]
    fn test_parse_whitespace_only_returns_error() {
        assert!(parse_datetime_input("   ").is_err());
    }

    #[test]
    fn test_parse_date_with_invalid_month_returns_error() {
        assert!(parse_datetime_input("2024-13-01").is_err());
    }

    #[test]
    fn test_parse_date_with_invalid_day_returns_error() {
        assert!(parse_datetime_input("2024-02-30").is_err());
    }
}
