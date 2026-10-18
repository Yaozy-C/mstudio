//! DSH str_replace_editor command semantics; persistence belongs to the host.
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub command: String,
    pub path: String,
    pub view_range: Option<[i64; 2]>,
    pub file_text: Option<String>,
    pub old_str: Option<String>,
    pub new_str: Option<String>,
    pub insert_line: Option<usize>,
}
impl Command {
    pub fn apply(&self, before: Option<&str>, null_replacement: bool) -> Result<String> {
        match self.command.as_str() {
            "create" => {
                ensure!(before.is_none(), "File already exists; use str_replace");
                Ok(self
                    .file_text
                    .clone()
                    .context("create requires file_text")?)
            }
            "str_replace" => {
                ensure!(
                    !null_replacement,
                    "new_str must be a string or omitted to delete the match"
                );
                let before = before.context("File not found; use create")?;
                let old = self
                    .old_str
                    .as_deref()
                    .context("str_replace requires old_str")?;
                ensure!(!old.is_empty(), "old_str must not be empty");
                let count = before.matches(old).count();
                ensure!(
                    count == 1,
                    "Expected one exact old_str match, found {count}; view the file and provide unique context"
                );
                Ok(before.replacen(old, self.new_str.as_deref().unwrap_or(""), 1))
            }
            "insert" => {
                let before = before.context("File not found; use create")?;
                let line = self.insert_line.context("insert requires insert_line")?;
                let text = self.new_str.as_deref().context("insert requires new_str")?;
                let mut lines = before.split('\n').collect::<Vec<_>>();
                ensure!(
                    line <= lines.len(),
                    "insert_line must be within [0, {}]",
                    lines.len()
                );
                lines.insert(line, text);
                Ok(lines.join("\n"))
            }
            _ => bail!("Unknown command; use view, create, str_replace or insert"),
        }
    }
}
pub fn view(path: &str, text: &str, range: Option<[i64; 2]>) -> Result<String> {
    let lines = text.split('\n').collect::<Vec<_>>();
    let total = lines.len() as i64;
    let [start, end] = range.unwrap_or([1, -1]);
    ensure!(
        (1..=total).contains(&start),
        "Invalid view_range start {start}; {path} has {total} lines. Use a start in [1, {total}]"
    );
    ensure!(
        end == -1 || (start..=total).contains(&end),
        "Invalid view_range end {end}; {path} has {total} lines. Use -1 for EOF or an end in [{start}, {total}]"
    );
    let end = if end == -1 { total } else { end };
    let mut result = format!("Content of {path} with line numbers (total {total} lines):\n");
    for (index, line) in lines
        .iter()
        .enumerate()
        .take(end as usize)
        .skip(start as usize - 1)
    {
        result.push_str(&format!("{:6}  {line}\n", index + 1));
    }
    if result.chars().count() > 16_000 {
        result = result.chars().take(16_000).collect();
        result.push_str("\n<response clipped>Read a smaller view_range to continue.\n");
    }
    Ok(result)
}
pub fn normalize(path: &str) -> Result<String> {
    ensure!(
        path.starts_with('/'),
        "path must be absolute, starting with /"
    );
    ensure!(
        path.len() <= 250 && !path.contains(['\\', '\0', ':']),
        "Invalid workspace path"
    );
    let trimmed = path.trim_end_matches('/');
    ensure!(
        trimmed.is_empty()
            || trimmed[1..]
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".."),
        "Invalid workspace path"
    );
    Ok(if trimmed.is_empty() {
        "/".into()
    } else {
        trimmed.into()
    })
}
