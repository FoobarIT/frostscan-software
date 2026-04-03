pub fn format_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let s = size as f64;

    if s >= TB {
        format!("{:.1} TB", s / TB)
    } else if s >= GB {
        format!("{:.1} GB", s / GB)
    } else if s >= MB {
        format!("{:.1} MB", s / MB)
    } else if s >= KB {
        format!("{:.1} KB", s / KB)
    } else {
        format!("{} B", size)
    }
}

#[cfg(test)]
mod tests {
    use super::format_size;

    #[test]
    fn formats_bytes_without_unit_conversion() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn formats_kilobytes_and_megabytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn formats_gigabytes_and_terabytes() {
        assert_eq!(format_size(1024_u64.pow(3)), "1.0 GB");
        assert_eq!(format_size(1024_u64.pow(4)), "1.0 TB");
    }
}
