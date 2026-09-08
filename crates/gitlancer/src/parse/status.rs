use crate::error::ParseError;
use crate::git::status::{StatusEntry, StatusFileEntry};

/// Parses porcelain-v2 `-z` output into raw status entries while preserving Git's machine-readable records.
pub fn parse_status_v2(stdout: &str) -> Result<Vec<StatusEntry>, ParseError> {
    let entries = stdout
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .filter(|entry| !entry.starts_with('#'))
        .map(|entry| StatusEntry {
            raw: entry.to_string(),
        })
        .collect::<Vec<_>>();

    Ok(entries)
}

/// Parses porcelain-v2 `-z` output into structured per-file staging entries for the review panel.
///
/// Each record is either a staged/unstaged change line (`1`/`2`/`u`) or an untracked line
/// (`?`). The two-letter `XY` status carries the index (staged) and worktree (unstaged) bits,
/// so a file is staged when its index bit is not `.`. Header lines starting with `#` are ignored.
pub fn parse_status_v2_entries(stdout: &str) -> Result<Vec<StatusFileEntry>, ParseError> {
    let mut entries = Vec::new();
    for record in stdout.split('\0').filter(|entry| !entry.is_empty()) {
        if record.starts_with('#') {
            continue;
        }
        let mut fields = record.split(' ');
        let Some(record_type) = fields.next() else {
            continue;
        };
        match record_type {
            "1" | "2" | "u" => {
                // `XY` is the second field: index bit (staged) then worktree bit (unstaged).
                let Some(xy) = fields.next() else {
                    continue;
                };
                // Skip the fixed metadata columns that follow `XY`; the remaining fields are
                // the path. Unmerged records (`u`) carry three stage hashes, hence one extra.
                let metadata_after_xy = match record_type {
                    "1" => 6,
                    "2" => 7,
                    "u" => 8,
                    _ => unreachable!("record types above are matched"),
                };
                for _ in 0..metadata_after_xy {
                    if fields.next().is_none() {
                        break;
                    }
                }
                let path = fields.collect::<Vec<_>>().join(" ");
                entries.push(StatusFileEntry {
                    path,
                    is_staged: xy.as_bytes().first() != Some(&b'.'),
                    is_untracked: false,
                });
            }
            "?" => {
                let path = fields.collect::<Vec<_>>().join(" ");
                entries.push(StatusFileEntry {
                    path,
                    is_staged: false,
                    is_untracked: true,
                });
            }
            _ => {}
        }
    }

    Ok(entries)
}
#[cfg(test)]
mod tests {
    use super::parse_status_v2_entries;
    use crate::git::status::StatusFileEntry;
    use pretty_assertions::assert_eq;

    /// Verifies staged, worktree-only, and untracked records map to the expected per-file state.
    #[test]
    fn parses_staged_unstaged_and_untracked_records() {
        let stdout = "1 M. N... 100644 100644 100644 abc abc a.txt\0? new.txt\0";
        let entries = parse_status_v2_entries(stdout).expect("parse status entries");

        assert_eq!(
            entries,
            vec![
                StatusFileEntry {
                    path: "a.txt".to_string(),
                    is_staged: true,
                    is_untracked: false,
                },
                StatusFileEntry {
                    path: "new.txt".to_string(),
                    is_staged: false,
                    is_untracked: true,
                },
            ]
        );
    }

    /// Verifies a worktree-only edit is reported as unstaged even when tracked.
    #[test]
    fn marks_worktree_only_changes_as_unstaged() {
        let stdout = "1 .M N... 100644 100644 100644 abc abc a.txt\0";
        let entries = parse_status_v2_entries(stdout).expect("parse status entries");

        assert_eq!(
            entries[0],
            StatusFileEntry {
                path: "a.txt".to_string(),
                is_staged: false,
                is_untracked: false,
            }
        );
    }

    /// Verifies porcelain header lines are ignored and do not surface as file entries.
    #[test]
    fn ignores_header_lines() {
        let stdout = "# branch.oid abc123\0? new.txt\0";
        let entries = parse_status_v2_entries(stdout).expect("parse status entries");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "new.txt");
    }
}
