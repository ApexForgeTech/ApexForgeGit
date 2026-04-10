/// 3-way merge engine with conflict detection and auto-resolve

#[derive(Debug, Clone, PartialEq)]
pub enum MergeResult {
    Clean(String),
    Conflict(String),
}

pub struct MergeEngine;

impl MergeEngine {
    /// 3-way merge: base (common ancestor), ours, theirs
    pub fn merge3(base: &str, ours: &str, theirs: &str) -> MergeResult {
        let base_lines: Vec<&str> = base.lines().collect();
        let ours_lines: Vec<&str> = ours.lines().collect();
        let theirs_lines: Vec<&str> = theirs.lines().collect();

        let mut result = Vec::new();
        let mut has_conflict = false;

        let max_len = base_lines.len().max(ours_lines.len()).max(theirs_lines.len());

        let mut bi = 0;
        let mut oi = 0;
        let mut ti = 0;

        while bi < max_len || oi < ours_lines.len() || ti < theirs_lines.len() {
            let b = base_lines.get(bi).copied();
            let o = ours_lines.get(oi).copied();
            let t = theirs_lines.get(ti).copied();

            match (b, o, t) {
                // All three same — no change
                (Some(bl), Some(ol), Some(tl)) if bl == ol && bl == tl => {
                    result.push(ol.to_string());
                    bi += 1; oi += 1; ti += 1;
                }
                // Ours changed, theirs same as base — take ours
                (Some(bl), Some(ol), Some(tl)) if bl == tl && bl != ol => {
                    result.push(ol.to_string());
                    bi += 1; oi += 1; ti += 1;
                }
                // Theirs changed, ours same as base — take theirs
                (Some(bl), Some(ol), Some(tl)) if bl == ol && bl != tl => {
                    result.push(tl.to_string());
                    bi += 1; oi += 1; ti += 1;
                }
                // Both changed same way — take either (they agree)
                (Some(_bl), Some(ol), Some(tl)) if ol == tl => {
                    result.push(ol.to_string());
                    bi += 1; oi += 1; ti += 1;
                }
                // Both changed differently — CONFLICT
                (Some(_bl), Some(ol), Some(tl)) => {
                    has_conflict = true;
                    result.push("<<<<<<< OURS".to_string());
                    result.push(ol.to_string());
                    result.push("=======".to_string());
                    result.push(tl.to_string());
                    result.push(">>>>>>> THEIRS".to_string());
                    bi += 1; oi += 1; ti += 1;
                }
                // Base ended, ours has extra lines
                (None, Some(ol), None) => {
                    result.push(ol.to_string());
                    oi += 1;
                }
                // Base ended, theirs has extra lines
                (None, None, Some(tl)) => {
                    result.push(tl.to_string());
                    ti += 1;
                }
                // Both added extra lines after base
                (None, Some(ol), Some(tl)) if ol == tl => {
                    result.push(ol.to_string());
                    oi += 1; ti += 1;
                }
                (None, Some(ol), Some(tl)) => {
                    has_conflict = true;
                    result.push("<<<<<<< OURS".to_string());
                    result.push(ol.to_string());
                    result.push("=======".to_string());
                    result.push(tl.to_string());
                    result.push(">>>>>>> THEIRS".to_string());
                    oi += 1; ti += 1;
                }
                // One side deleted line or ended
                _ => {
                    if let Some(ol) = o { result.push(ol.to_string()); oi += 1; }
                    if let Some(tl) = t { result.push(tl.to_string()); ti += 1; }
                    bi += 1;
                }
            }
        }

        let merged = result.join("\n");
        if has_conflict {
            MergeResult::Conflict(merged)
        } else {
            MergeResult::Clean(merged)
        }
    }

    /// Check if merged text has conflict markers
    pub fn has_conflicts(text: &str) -> bool {
        text.contains("<<<<<<< OURS") && text.contains(">>>>>>> THEIRS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_merge_no_changes() {
        let base = "line1\nline2\nline3";
        let result = MergeEngine::merge3(base, base, base);
        assert_eq!(result, MergeResult::Clean(base.to_string()));
    }

    #[test]
    fn test_clean_merge_one_side_changed() {
        let base = "line1\nline2\nline3";
        let ours = "line1\nMODIFIED\nline3";
        let result = MergeEngine::merge3(base, ours, base);
        assert_eq!(result, MergeResult::Clean(ours.to_string()));
    }

    #[test]
    fn test_conflict() {
        let base = "line1\nline2\nline3";
        let ours = "line1\nOURS_CHANGE\nline3";
        let theirs = "line1\nTHEIRS_CHANGE\nline3";
        let result = MergeEngine::merge3(base, ours, theirs);
        match result {
            MergeResult::Conflict(text) => {
                assert!(text.contains("<<<<<<< OURS"));
                assert!(text.contains("OURS_CHANGE"));
                assert!(text.contains("THEIRS_CHANGE"));
                assert!(text.contains(">>>>>>> THEIRS"));
            }
            _ => panic!("Expected conflict"),
        }
    }

    #[test]
    fn test_both_add_same_line() {
        let base = "line1";
        let ours = "line1\nnew_line";
        let theirs = "line1\nnew_line";
        let result = MergeEngine::merge3(base, ours, theirs);
        assert_eq!(result, MergeResult::Clean("line1\nnew_line".to_string()));
    }
}
