//! Scoped DSH-style file editor. Every successful mutation is already saved.
use super::{
    File,
    editor_commands::{self, Command},
    files,
};
use crate::database::Store;
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const TOOL_NAME: &str = "str_replace_editor";
pub const MAX_BYTES: usize = 2_000_000;
pub struct Editor {
    pub file: File,
    entries: BTreeMap<String, File>,
    created: BTreeSet<String>,
    known_models: BTreeSet<String>,
}
impl Editor {
    pub fn new(file: &File, all: &[File]) -> Self {
        let current = if file.kind == "shot" {
            "/scene.js"
        } else {
            "/model.js"
        };
        let mut entries = BTreeMap::from([(current.into(), file.clone())]);
        for model in all
            .iter()
            .filter(|m| m.kind == "model" && file.asset_ids.contains(&m.id))
        {
            entries.insert(format!("/models/{}.js", model.id), model.clone());
        }
        Self {
            file: file.clone(),
            entries,
            created: BTreeSet::new(),
            known_models: all
                .iter()
                .filter(|m| m.kind == "model")
                .map(|m| m.id.clone())
                .collect(),
        }
    }
    pub fn execute(
        &mut self,
        store: &Store,
        project: &str,
        name: &str,
        input: Value,
    ) -> Result<String> {
        ensure!(name == TOOL_NAME, "Only str_replace_editor is available");
        let null_replacement = input.get("new_str").is_some_and(Value::is_null);
        let c: Command = serde_json::from_value(input).context("Invalid editor arguments")?;
        let path = editor_commands::normalize(&c.path)?;
        if c.command == "view" && (path == "/" || path == "/models") {
            ensure!(
                c.view_range.is_none(),
                "view_range is not allowed for a directory"
            );
            return Ok(self
                .entries
                .keys()
                .filter(|p| path == "/" || p.starts_with("/models/"))
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"));
        }
        let old = self.entries.get(&path).cloned();
        let mut db = store.db.lock().unwrap();
        let tx = db.transaction()?;
        let mut source_exists = false;
        let current = if let Some(observed) = &old {
            let mut current =
                files::read(&tx, project, &observed.id)?.context("File was deleted")?;
            // Read the actual source mirror, including external file edits.
            match std::fs::read_to_string(files::path(store, project, &current)?) {
                Ok(text) => {
                    current.code = text;
                    source_exists = true;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            Some(current)
        } else {
            None
        };
        if c.command == "view" {
            let current = current.context("File not found")?;
            ensure!(source_exists, "File not found; use create");
            let result = editor_commands::view(&path, &current.code, c.view_range)?;
            if current.id == self.file.id {
                self.file = current.clone();
            }
            self.entries.insert(path, current);
            return Ok(result);
        }
        let is_current = old.as_ref().is_some_and(|f| f.id == self.file.id);
        ensure!(
            old.is_none() || is_current || self.created.contains(&path),
            "Existing shared models are read-only; open that model to edit it"
        );
        let alias = if old.is_none() {
            ensure!(
                self.file.kind == "shot",
                "Only the current model file is writable"
            );
            let alias =
                model_name(&path).context("Only /scene.js and /models/<name>.js are writable")?;
            ensure!(
                !self.known_models.contains(alias),
                "This model exists; select it in shared models before reuse"
            );
            Some(alias.to_owned())
        } else {
            None
        };
        if let (Some(observed), Some(current)) = (&old, &current) {
            ensure!(
                current.revision == observed.revision
                    && (!source_exists || current.code == observed.code),
                "File changed since it was observed; view the file and retry the edit"
            );
        }
        let before = current
            .as_ref()
            // A host-created empty source is an uninitialized file, safe to fill with create.
            // Revision and disk checks above still reject concurrent edits.
            .filter(|file| {
                source_exists
                    && !(is_current && file.code.trim().is_empty() && c.command == "create")
            })
            .map(|f| f.code.as_str());
        let next = c.apply(before, null_replacement)?;
        ensure!(next.len() <= MAX_BYTES, "File exceeds 2 MB");
        let mut target = current.clone().unwrap_or_else(|| File {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "model".into(),
            name: alias.clone().unwrap(),
            description: String::new(),
            code: String::new(),
            revision: 0,
            asset_ids: vec![],
            bindings: BTreeMap::new(),
            duration: 5.,
            width: 720,
            height: 720,
            fps: 24,
        });
        target.code = next;
        let expected = current.as_ref().map_or(0, |f| f.revision);
        let mut shot = None;
        if let Some(alias) = &alias {
            let mut latest =
                files::read(&tx, project, &self.file.id)?.context("Shot was deleted")?;
            ensure!(
                latest.revision == self.file.revision,
                "Shot changed; view /scene.js and retry"
            );
            ensure!(
                std::fs::read_to_string(files::path(store, project, &latest)?)? == self.file.code,
                "Shot changed on disk; view /scene.js and retry"
            );
            latest.asset_ids.push(target.id.clone());
            latest.bindings.insert(alias.clone(), target.id.clone());
            shot = Some(latest);
        }
        files::validate_target(&tx, project, &target)?;
        if let Some(shot) = &shot {
            tx.execute(
                "INSERT INTO previs_files VALUES(?1,?2,?3,?4)",
                rusqlite::params![
                    project,
                    target.id,
                    target.kind,
                    serde_json::to_string(&target)?
                ],
            )?;
            files::validate_target(&tx, project, shot)?;
        }
        // The existing persistence layer performs version checks and atomic source replacement.
        let saved = files::save(&tx, store, project, target, expected)?;
        let shot = shot
            .map(|s| {
                let revision = s.revision;
                files::save(&tx, store, project, s, revision)
            })
            .transpose()?;
        tx.commit()?;
        if let Some(shot) = shot {
            self.file = shot.clone();
            self.entries.insert("/scene.js".into(), shot);
            self.created.insert(path.clone());
        } else if saved.id == self.file.id {
            self.file = saved.clone();
        }
        self.entries.insert(path.clone(), saved);
        Ok(format!("The file {path} has been saved successfully."))
    }
}
fn model_name(path: &str) -> Option<&str> {
    let name = path.strip_prefix("/models/")?.strip_suffix(".js")?;
    (!name.trim().is_empty()
        && name.len() <= 200
        && !name.contains(['/', '.', '\n', '\r'])
        && !name.chars().any(char::is_control))
    .then_some(name)
}
