//! Marker-delimited managed blocks: the one splice `fissile init` performs on
//! files someone else owns — the agent block and the hook (§FS-002-init.4,
//! §FS-002-init.6). Explicit markers keep what a user writes below the block.

use std::path::Path;

use crate::init::{Action, InitError};

/// A managed block: how to find it, what version this build writes, and the
/// body it writes — marker lines included, no trailing newline.
pub struct Block<'a> {
    pub begin_prefix: &'a str,
    pub end_prefix: &'a str,
    pub version: u32,
    pub body: &'a str,
    /// The in-block line carrying the version, by its prefix through `(v`; it
    /// also finds a pre-marker block, whose only boundary that heading was.
    /// `None` reads the version from the begin marker instead (§FS-002-init.4).
    pub version_heading: Option<&'a str>,
}

impl Block<'_> {
    /// The file's new contents after appending, replacing, or leaving the block.
    pub fn apply(&self, existing: &str, path: &Path) -> Result<(String, Action), InitError> {
        let lines: Vec<&str> = existing.lines().collect();

        let Some(span) = self.locate(&lines, path)? else {
            return Ok((append_to(existing, self.body), Action::Appended));
        };

        let result = splice(&lines, span, self.body);
        let action = if result == existing {
            Action::Exists
        } else {
            Action::Updated
        };
        Ok((result, action))
    }

    /// The line range the block occupies, or `None` when the file has no block.
    /// A version this build cannot write is an error, not a span to overwrite.
    fn locate(
        &self,
        lines: &[&str],
        path: &Path,
    ) -> Result<Option<std::ops::Range<usize>>, InitError> {
        if let Some(begin) = lines.iter().position(|line| self.is_begin(line)) {
            // One past the matching end marker; a truncated block — a begin
            // marker with no end — falls back to §FS-002-init.4's rule for the
            // kind of file this block lives in.
            let end = lines[begin + 1..]
                .iter()
                .position(|line| self.is_end(line))
                .map(|offset| begin + 1 + offset + 1)
                .unwrap_or_else(|| self.truncated_span_end(lines, begin));
            match self.version_in(&lines[begin..end]) {
                Some(version) => self.check_version(version, path)?,
                // Our markers around a heading this build cannot read: a newer
                // generation renamed it, and the markers carry no version to
                // fall back on, so this is not ours to overwrite (§FS-002-init.4).
                None => {
                    return Err(InitError::UnsupportedBlock {
                        path: path.to_path_buf(),
                        version: None,
                    });
                }
            }
            return Ok(Some(begin..end));
        }

        // No markers: a block from before they existed, bounded by its heading.
        let Some(prefix) = self.version_heading else {
            return Ok(None);
        };
        let Some(start) = lines
            .iter()
            .position(|line| line.trim_end().starts_with(prefix))
        else {
            return Ok(None);
        };
        let version = version_after(lines[start], prefix).unwrap_or(self.version);
        self.check_version(version, path)?;
        Ok(Some(start..self.heading_span_end(lines, start)))
    }

    /// The version a located block declares: from its heading when it carries
    /// one, else from the begin marker. `None` when a block that states its
    /// version in a heading has none this build can read — the one case where
    /// "assume current" would silently downgrade a newer block.
    fn version_in(&self, span: &[&str]) -> Option<u32> {
        let Some(prefix) = self.version_heading else {
            return Some(version_after(span[0], self.begin_prefix).unwrap_or(self.version));
        };
        span.iter().find_map(|line| version_after(line, prefix))
    }

    /// Where a begin marker with no end marker ends: the heading rule below for
    /// Markdown, end of file for a marker-only block, whose file has no headings
    /// to find — in a shell hook every `# ` comment reads as one (§FS-002-init.4).
    fn truncated_span_end(&self, lines: &[&str], start: usize) -> usize {
        if self.version_heading.is_none() {
            return lines.len();
        }
        self.heading_span_end(lines, start)
    }

    /// Where a heading-bounded span starting at `start` ends: the next H1 or
    /// H2, or end of file. The block's own heading does not end it.
    fn heading_span_end(&self, lines: &[&str], start: usize) -> usize {
        lines
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, line)| is_heading(line) && !self.is_own_heading(line))
            .map_or(lines.len(), |(index, _)| index)
    }

    fn is_own_heading(&self, line: &str) -> bool {
        self.version_heading
            .is_some_and(|prefix| line.trim_end().starts_with(prefix))
    }

    fn check_version(&self, version: u32, path: &Path) -> Result<(), InitError> {
        if version > self.version {
            return Err(InitError::UnsupportedBlock {
                path: path.to_path_buf(),
                version: Some(version),
            });
        }
        Ok(())
    }

    fn is_begin(&self, line: &str) -> bool {
        line.trim_start().starts_with(self.begin_prefix)
    }

    fn is_end(&self, line: &str) -> bool {
        line.trim_start().starts_with(self.end_prefix)
    }
}

/// The block appended below whatever the file already held.
fn append_to(existing: &str, body: &str) -> String {
    let mut result = existing.trim_end().to_owned();
    if !result.is_empty() {
        result.push_str("\n\n");
    }
    result.push_str(body);
    result.push('\n');
    result
}

/// Replace `span` with `body`, keeping every line outside it.
fn splice(lines: &[&str], span: std::ops::Range<usize>, body: &str) -> String {
    let before = lines[..span.start].join("\n");
    let after = lines[span.end..].join("\n");

    let mut result = String::new();
    if !before.is_empty() {
        result.push_str(&before);
        result.push('\n');
    }
    result.push_str(body);
    result.push('\n');
    if !after.is_empty() {
        result.push_str(&after);
        result.push('\n');
    }
    result
}

fn is_heading(line: &str) -> bool {
    let trimmed = line.trim_start();
    (trimmed.starts_with("# ") || trimmed.starts_with("## ")) && !trimmed.starts_with("### ")
}

/// The version digits following `prefix` on a marker or heading line.
fn version_after(line: &str, prefix: &str) -> Option<u32> {
    let rest = line.trim_start().strip_prefix(prefix)?;
    let digits: String = rest
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    digits.parse().ok()
}
