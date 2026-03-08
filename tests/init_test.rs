use sentinel::config::expand_tilde;

#[test]
fn test_expand_tilde() {
    let expanded = expand_tilde("~/some/path");
    // Should not start with ~ anymore (replaced with $HOME)
    assert!(!expanded.starts_with('~'), "~ should be expanded, got: {expanded}");
    assert!(expanded.ends_with("/some/path"));

    // Just "~" alone should expand too
    let home_only = expand_tilde("~");
    assert!(!home_only.starts_with('~'), "~ alone should expand, got: {home_only}");
    assert!(!home_only.is_empty());
}

#[test]
fn test_expand_tilde_no_tilde() {
    // Paths without ~ should be unchanged
    assert_eq!(expand_tilde("/usr/local/bin"), "/usr/local/bin");
    assert_eq!(expand_tilde("relative/path"), "relative/path");
    assert_eq!(expand_tilde(""), "");
    // Tilde in the middle should not be expanded
    assert_eq!(expand_tilde("/home/user/~backup"), "/home/user/~backup");
}
