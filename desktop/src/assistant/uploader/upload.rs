use super::{LIFETIME, REFRESH_MARGIN, Result, Source, Uploaded, error, now};
use rig_core::http_client::Error;

pub struct UploadRequest<'a> {
    pub client: &'a reqwest::Client,
    pub root: &'a str,
    pub key: &'a str,
}

#[derive(serde::Deserialize)]
struct Receipt {
    id: String,
    object: String,
    bytes: u64,
    created_at: i64,
    filename: String,
    purpose: String,
    expires_at: Option<i64>,
}

// Reusable protocol implementation; each platform chooses its upload operation.
pub(super) async fn openai_file(
    backend: &dyn super::backends::Backend,
    request: UploadRequest<'_>,
    source: &Source,
) -> Result<Uploaded> {
    let boundary = format!("mstudio-{}", mstudio::media::id());
    backend.validate(source)?;
    let extension = source.mime.split('/').next_back().unwrap();
    let purpose = backend.purpose(source);
    let filename = format!("mstudio-{}.{}", source.variant, extension);
    let mut body = Vec::new();
    for (name, value) in [
        ("purpose", purpose.into()),
        ("expires_after[anchor]", "created_at".into()),
        ("expires_after[seconds]", LIFETIME.to_string()),
    ] {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {}\r\n\r\n", source.mime).as_bytes());
    body.extend_from_slice(&source.bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = request
        .client
        .post(format!("{}/files", request.root))
        .bearer_auth(request.key)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .map_err(Error::instance)?;
    let status = response.status();
    let headers = response.headers().clone();
    let raw = response.bytes().await.map_err(Error::instance)?;
    if !status.is_success() {
        return Err(Error::non_success_with_details(
            status,
            headers,
            String::from_utf8_lossy(&raw).into_owned(),
        ));
    }
    let record: Receipt = serde_json::from_slice(&raw).map_err(Error::instance)?;
    let expires = record
        .expires_at
        .unwrap_or(record.created_at.saturating_add(LIFETIME));
    if record.object != "file"
        || !record.id.starts_with(backend.id_prefix())
        || record.id.len() <= backend.id_prefix().len()
        || record.id.len() > 256
        || !record
            .id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        || record.bytes != source.bytes.len() as u64
        || record.purpose != purpose
        || record.filename != filename
        || record.created_at <= 0
        || expires <= now() + REFRESH_MARGIN
    {
        return Err(error("上传平台返回无效的文件回执"));
    }
    Ok(Uploaded {
        id: record.id,
        bytes: record.bytes,
        expires_at: expires,
    })
}
