use similar::{ChangeTag, TextDiff};

pub fn compute_diff(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut result = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        result.push_str(&format!("{}{}", sign, change));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_diff_addition() {
        let old = "line 1\nline 2\n";
        let new = "line 1\nline 2\nline 3\n";
        let diff = compute_diff(old, new);

        assert!(diff.contains(" line 1"));
        assert!(diff.contains(" line 2"));
        assert!(diff.contains("+line 3"));
    }

    #[test]
    fn test_compute_diff_deletion() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nline 3\n";
        let diff = compute_diff(old, new);

        assert!(diff.contains(" line 1"));
        assert!(diff.contains("-line 2"));
        assert!(diff.contains(" line 3"));
    }

    #[test]
    fn test_compute_diff_modification() {
        let old = "hello world\n";
        let new = "hello Synthia\n";
        let diff = compute_diff(old, new);

        assert!(diff.contains("-hello world"));
        assert!(diff.contains("+hello Synthia"));
    }

    #[test]
    fn test_compute_diff_no_change() {
        let old = "same\n";
        let new = "same\n";
        let diff = compute_diff(old, new);

        assert!(diff.contains(" same"));
        assert!(!diff.contains("+"));
        assert!(!diff.contains("-"));
    }
}
