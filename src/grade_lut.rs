//! Disposable LUT cache; only the editable recipe belongs in the project document.
use crate::grading::Grade;
use anyhow::Result;
use std::{
    hash::{Hash, Hasher},
    io::Write,
    path::PathBuf,
};
pub fn bake(grade: &Grade) -> Result<PathBuf> {
    grade.validate()?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    ("mlight-sdr-grade-v1", serde_json::to_string(grade)?).hash(&mut hash);
    let root = std::env::temp_dir().join("mstudio-grade-luts-v1");
    std::fs::create_dir_all(&root)?;
    let target = root.join(format!("{:x}.cube", hash.finish()));
    if target.is_file() {
        return Ok(target);
    }
    let partial = root.join(format!("{}.partial", crate::media::id()));
    let result = (|| -> Result<()> {
        let mut out = std::io::BufWriter::new(std::fs::File::create(&partial)?);
        writeln!(
            out,
            "TITLE \"Mstudio editable SDR grade\"\nLUT_3D_SIZE 33\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1"
        )?;
        for b in 0..33 {
            for g in 0..33 {
                for r in 0..33 {
                    let p = grade.pixel([r as f64 / 32., g as f64 / 32., b as f64 / 32.]);
                    writeln!(out, "{:.8} {:.8} {:.8}", p[0], p[1], p[2])?;
                }
            }
        }
        out.flush()?;
        drop(out);
        std::fs::rename(&partial, &target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(partial);
    }
    result?;
    Ok(target)
}
