/// A collection maps 1:1 to a stathost bucket, so it must be a single safe segment.
pub fn is_valid_collection(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

/// A document slug may be nested (`eu/volvo`); every segment must be safe and
/// free of path traversal so it cannot escape the bucket in stathost.
pub fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.split('/').all(is_valid_segment)
}

fn is_valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collections() {
        assert!(is_valid_collection("cars"));
        assert!(is_valid_collection("my-cars_2"));
        assert!(!is_valid_collection(""));
        assert!(!is_valid_collection("cars/boats"));
        assert!(!is_valid_collection(".."));
        assert!(!is_valid_collection("car.s"));
    }

    #[test]
    fn slugs() {
        assert!(is_valid_slug("volvo"));
        assert!(is_valid_slug("eu/volvo"));
        assert!(is_valid_slug("v1.2.3"));
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("../secret"));
        assert!(!is_valid_slug("eu//volvo"));
        assert!(!is_valid_slug("eu/../volvo"));
    }
}
