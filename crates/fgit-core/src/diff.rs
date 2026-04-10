/// Myers diff algorithm for line-level and word-level diffing
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub old_lineno: Option<usize>,
    pub new_lineno: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

impl fmt::Display for DiffHunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "@@ -{},{} +{},{} @@",
            self.old_start, self.old_count,
            self.new_start, self.new_count)?;
        for line in &self.lines {
            let prefix = match line.line_type {
                DiffLineType::Context => " ",
                DiffLineType::Added => "+",
                DiffLineType::Removed => "-",
            };
            writeln!(f, "{}{}", prefix, line.content)?;
        }
        Ok(())
    }
}

pub struct DiffEngine;

impl DiffEngine {
    /// Compute line-level diff between two texts using Myers algorithm
    pub fn diff(old: &str, new: &str) -> Vec<DiffHunk> {
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();
        let edit_script = Self::myers_diff(&old_lines, &new_lines);
        Self::build_hunks(&edit_script, &old_lines, &new_lines, 3)
    }

    /// Word-level diff for a single line pair (ApexForge Git exclusive)
    pub fn word_diff(old: &str, new: &str) -> Vec<(DiffLineType, String)> {
        let old_words: Vec<&str> = old.split_whitespace().collect();
        let new_words: Vec<&str> = new.split_whitespace().collect();
        let edits = Self::myers_diff(&old_words, &new_words);
        let mut result = Vec::new();
        for edit in edits {
            match edit {
                Edit::Equal(s) => result.push((DiffLineType::Context, s.to_string())),
                Edit::Insert(s) => result.push((DiffLineType::Added, s.to_string())),
                Edit::Delete(s) => result.push((DiffLineType::Removed, s.to_string())),
            }
        }
        result
    }

    /// Core Myers diff — returns a sequence of edits
    fn myers_diff<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Edit<'a>> {
        let n = old.len();
        let m = new.len();

        if n == 0 && m == 0 {
            return Vec::new();
        }
        if n == 0 {
            return new.iter().map(|s| Edit::Insert(s)).collect();
        }
        if m == 0 {
            return old.iter().map(|s| Edit::Delete(s)).collect();
        }

        let max = n + m;
        let size = 2 * max + 1;
        let mut v = vec![0i64; size];
        let mut trace: Vec<Vec<i64>> = Vec::new();

        let offset = max as i64;

        for d in 0..=(max as i64) {
            trace.push(v.clone());
            let mut k = -d;
            while k <= d {
                let idx = (k + offset) as usize;
                let x: i64;
                if k == -d || (k != d && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize]) {
                    x = v[(k + 1 + offset) as usize];
                } else {
                    x = v[(k - 1 + offset) as usize] + 1;
                }
                let mut xi = x;
                let mut y = x - k;

                while (xi as usize) < n && (y as usize) < m && old[xi as usize] == new[y as usize] {
                    xi += 1;
                    y += 1;
                }

                v[idx] = xi;

                if (xi as usize) >= n && (y as usize) >= m {
                    return Self::backtrack(&trace, old, new, offset);
                }

                k += 2;
            }
        }

        // Fallback — should not reach here
        let mut result = Vec::new();
        for s in old { result.push(Edit::Delete(s)); }
        for s in new { result.push(Edit::Insert(s)); }
        result
    }

    fn backtrack<'a>(trace: &[Vec<i64>], old: &[&'a str], new: &[&'a str], offset: i64) -> Vec<Edit<'a>> {
        let mut edits = Vec::new();
        let mut x = old.len() as i64;
        let mut y = new.len() as i64;

        for d in (0..trace.len()).rev() {
            let v = &trace[d];
            let k = x - y;
            let d_i64 = d as i64;

            if d == 0 {
                // At d=0, the only valid point is (0,0)
                // Walk diagonal from (x,y) back to (0,0)
                while x > 0 && y > 0 {
                    x -= 1;
                    y -= 1;
                    edits.push(Edit::Equal(old[x as usize]));
                }
                break;
            }

            let prev_k: i64;
            let k_minus_1_idx = (k - 1 + offset) as usize;
            let k_plus_1_idx = (k + 1 + offset) as usize;

            if k == -d_i64
                || (k != d_i64
                    && k_minus_1_idx < v.len()
                    && k_plus_1_idx < v.len()
                    && v[k_minus_1_idx] < v[k_plus_1_idx])
            {
                prev_k = k + 1;
            } else {
                prev_k = k - 1;
            }

            let prev_k_idx = (prev_k + offset) as usize;
            if prev_k_idx >= v.len() {
                break;
            }
            let prev_x = v[prev_k_idx];
            let prev_y = prev_x - prev_k;

            // Diagonal (equal)
            while x > prev_x && y > prev_y {
                x -= 1;
                y -= 1;
                edits.push(Edit::Equal(old[x as usize]));
            }

            if d > 0 {
                if x > prev_x {
                    x -= 1;
                    edits.push(Edit::Delete(old[x as usize]));
                } else if y > prev_y {
                    y -= 1;
                    edits.push(Edit::Insert(new[y as usize]));
                }
            }

            if x == 0 && y == 0 {
                break;
            }
        }

        edits.reverse();
        edits
    }

    fn build_hunks(edits: &[Edit], _old_lines: &[&str], _new_lines: &[&str], context: usize) -> Vec<DiffHunk> {
        if edits.is_empty() {
            return Vec::new();
        }

        let mut hunks = Vec::new();
        let mut current_lines: Vec<DiffLine> = Vec::new();
        let mut old_line = 1usize;
        let mut new_line = 1usize;
        let mut hunk_old_start = 0usize;
        let mut hunk_new_start = 0usize;
        let mut in_hunk = false;
        let mut trailing_context = 0usize;

        for edit in edits {
            match edit {
                Edit::Equal(_s) => {
                    if in_hunk {
                        trailing_context += 1;
                        current_lines.push(DiffLine {
                            line_type: DiffLineType::Context,
                            content: _s.to_string(),
                            old_lineno: Some(old_line),
                            new_lineno: Some(new_line),
                        });
                        if trailing_context >= context * 2 {
                            // Close this hunk
                            let hunk = DiffHunk {
                                old_start: hunk_old_start,
                                old_count: current_lines.iter()
                                    .filter(|l| l.line_type != DiffLineType::Added).count(),
                                new_start: hunk_new_start,
                                new_count: current_lines.iter()
                                    .filter(|l| l.line_type != DiffLineType::Removed).count(),
                                lines: current_lines.clone(),
                            };
                            hunks.push(hunk);
                            current_lines.clear();
                            in_hunk = false;
                        }
                    }
                    old_line += 1;
                    new_line += 1;
                }
                Edit::Delete(_s) => {
                    trailing_context = 0;
                    if !in_hunk {
                        in_hunk = true;
                        hunk_old_start = old_line.saturating_sub(context);
                        hunk_new_start = new_line.saturating_sub(context);
                    }
                    current_lines.push(DiffLine {
                        line_type: DiffLineType::Removed,
                        content: _s.to_string(),
                        old_lineno: Some(old_line),
                        new_lineno: None,
                    });
                    old_line += 1;
                }
                Edit::Insert(_s) => {
                    trailing_context = 0;
                    if !in_hunk {
                        in_hunk = true;
                        hunk_old_start = old_line.saturating_sub(context);
                        hunk_new_start = new_line.saturating_sub(context);
                    }
                    current_lines.push(DiffLine {
                        line_type: DiffLineType::Added,
                        content: _s.to_string(),
                        old_lineno: None,
                        new_lineno: Some(new_line),
                    });
                    new_line += 1;
                }
            }
        }

        if !current_lines.is_empty() {
            hunks.push(DiffHunk {
                old_start: hunk_old_start,
                old_count: current_lines.iter()
                    .filter(|l| l.line_type != DiffLineType::Added).count(),
                new_start: hunk_new_start,
                new_count: current_lines.iter()
                    .filter(|l| l.line_type != DiffLineType::Removed).count(),
                lines: current_lines,
            });
        }

        hunks
    }
}

#[derive(Debug, Clone)]
enum Edit<'a> {
    Equal(&'a str),
    Insert(&'a str),
    Delete(&'a str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_no_diff() {
        let hunks = DiffEngine::diff("hello\nworld\n", "hello\nworld\n");
        assert!(hunks.is_empty() || hunks.iter().all(|h|
            h.lines.iter().all(|l| l.line_type == DiffLineType::Context)));
    }

    #[test]
    fn test_added_line() {
        let hunks = DiffEngine::diff("line1\n", "line1\nline2\n");
        assert!(!hunks.is_empty());
        let has_add = hunks.iter().any(|h|
            h.lines.iter().any(|l| l.line_type == DiffLineType::Added));
        assert!(has_add);
    }

    #[test]
    fn test_removed_line() {
        let hunks = DiffEngine::diff("line1\nline2\n", "line1\n");
        let has_remove = hunks.iter().any(|h|
            h.lines.iter().any(|l| l.line_type == DiffLineType::Removed));
        assert!(has_remove);
    }

    #[test]
    fn test_word_diff() {
        let result = DiffEngine::word_diff("hello world", "hello rust world");
        assert!(result.iter().any(|r| r.0 == DiffLineType::Added && r.1 == "rust"));
    }
}
