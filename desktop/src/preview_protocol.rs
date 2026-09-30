//! Private, bounded IPC between the app and its bundled preview worker.
use crate::ges_engine::Status;
use mstudio::preview_ges::Plan;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum Command {
    Play,
    Pause,
    Seek { frame: i32 },
    Rate { rate: f64 },
    Close,
}
impl Command {
    pub fn engine_args(&self) -> Result<(&'static str, i32), String> {
        Ok(match self {
            Self::Play => ("play", 0),
            Self::Pause => ("pause", 0),
            Self::Seek { frame } => ("seek", *frame),
            Self::Close => ("close", 0),
            Self::Rate { rate } => {
                if !rate.is_finite() || !(0.25..=2.).contains(rate) || (rate * 4.).fract() != 0. {
                    return Err("播放倍速必须为 0.25 至 2 倍，间隔 0.25".into());
                }
                ("rate", (rate * 4.) as i32)
            }
        })
    }
}
#[derive(Serialize, Deserialize)]
pub enum Request {
    Open(Plan),
    Control { id: u64, command: Command },
}
#[derive(Serialize, Deserialize)]
pub enum Event {
    Ready(Result<Status, String>),
    Snapshot(Status),
    Reply {
        id: u64,
        result: Result<Status, String>,
    },
}
const MAX_JSON: usize = 8 * 1024 * 1024;
const MAX_FRAME: usize = 16 + 3840 * 3840 * 4;
fn read_block(reader: &mut impl Read, maximum: usize) -> std::io::Result<Vec<u8>> {
    let mut size = [0; 4];
    reader.read_exact(&mut size)?;
    let size = u32::from_le_bytes(size) as usize;
    if size > maximum {
        return Err(std::io::Error::other("Preview IPC packet exceeds limit"));
    }
    let mut bytes = vec![0; size];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}
pub fn read<T: serde::de::DeserializeOwned>(
    reader: &mut impl Read,
) -> std::io::Result<(T, Vec<u8>)> {
    let json = read_block(reader, MAX_JSON)?;
    let frame = read_block(reader, MAX_FRAME)?;
    Ok((serde_json::from_slice(&json)?, frame))
}
pub fn write<T: Serialize>(
    writer: &mut impl Write,
    value: &T,
    frame: &[u8],
) -> std::io::Result<()> {
    let json = serde_json::to_vec(value)?;
    if json.len() > MAX_JSON || frame.len() > MAX_FRAME {
        return Err(std::io::Error::other("Preview IPC packet exceeds limit"));
    }
    writer.write_all(&(json.len() as u32).to_le_bytes())?;
    writer.write_all(&json)?;
    writer.write_all(&(frame.len() as u32).to_le_bytes())?;
    writer.write_all(frame)?;
    writer.flush()
}
