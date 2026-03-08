/// Route a tool name to the module that should handle it, based on prefix.
///
/// Returns a string identifying the target module. When domain modules are
/// built out, this will drive actual dispatch; for now it is used for
/// introspection and testing.
pub fn route_tool(name: &str) -> &'static str {
    // Order doesn't matter -- we match the first prefix that fits.
    // Each arm strips the prefix and delegates to the appropriate module path.
    if name.starts_with("qa_") {
        "domain::qa"
    } else if name.starts_with("eyes_") {
        "domain::eyes"
    } else if name.starts_with("lineage_") {
        "orchestration::lineage"
    } else if name.starts_with("hive_") {
        "orchestration::hive"
    } else if name.starts_with("skill_") {
        "orchestration::skills"
    } else if name.starts_with("opencheir_") {
        "orchestration::supervisor"
    } else if name.starts_with("enforcer_") {
        "orchestration::enforcer"
    } else if name.starts_with("pattern_") {
        "orchestration::patterns"
    } else if name.starts_with("search_") {
        "store::search"
    } else if name.starts_with("doc_") {
        "store::documents"
    } else if name.starts_with("word_") {
        "proxy::word-document-server"
    } else if name.starts_with("mermaid_") {
        "proxy::mermaid-kroki"
    } else if name.starts_with("puppeteer_") {
        "proxy::puppeteer"
    } else if name.starts_with("threejs_") {
        "proxy::threejs"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_all_prefixes() {
        assert_eq!(route_tool("qa_check_fonts"), "domain::qa");
        assert_eq!(route_tool("eyes_capture"), "domain::eyes");
        assert_eq!(route_tool("lineage_track"), "orchestration::lineage");
        assert_eq!(route_tool("hive_orchestrate"), "orchestration::hive");
        assert_eq!(route_tool("skill_list"), "orchestration::skills");
        assert_eq!(route_tool("opencheir_status"), "orchestration::supervisor");
        assert_eq!(route_tool("enforcer_check"), "orchestration::enforcer");
        assert_eq!(route_tool("pattern_analyze"), "orchestration::patterns");
        assert_eq!(route_tool("search_documents"), "store::search");
        assert_eq!(route_tool("doc_parse"), "store::documents");
        assert_eq!(route_tool("word_add_paragraph"), "proxy::word-document-server");
        assert_eq!(route_tool("mermaid_render"), "proxy::mermaid-kroki");
        assert_eq!(route_tool("puppeteer_click"), "proxy::puppeteer");
        assert_eq!(route_tool("threejs_show"), "proxy::threejs");
        assert_eq!(route_tool("unknown_tool"), "unknown");
    }
}
