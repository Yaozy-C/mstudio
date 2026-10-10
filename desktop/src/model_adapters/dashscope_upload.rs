use super::{configuration, endpoint, response};
use crate::jobs::client;
use anyhow::{Context, Result, ensure};
use serde_json::Value;

pub async fn upload(
    key: &str,
    base: &str,
    model: &str,
    bytes: Vec<u8>,
    mime: &str,
) -> Result<String> {
    let client = client()?;
    let url = endpoint(base)?.join(
        configuration()?["uploadPath"]
            .as_str()
            .context("缺少上传路径")?,
    )?;
    let policy = response(
        client
            .get(url)
            .bearer_auth(key)
            .query(&[("action", "getPolicy"), ("model", model)])
            .send()
            .await?,
        "reference_upload_initiate",
    )
    .await?;
    let data = &policy["data"];
    let field = |name: &str| -> Result<String> {
        Ok(data[name].as_str().context("上传凭证字段缺失")?.into())
    };
    let host = reqwest::Url::parse(&field("upload_host")?)?;
    ensure!(
        host.scheme() == "https"
            && host.username().is_empty()
            && host.password().is_none()
            && host
                .host_str()
                .is_some_and(|h| h.ends_with(".aliyuncs.com")),
        "上传地址必须来自阿里云 OSS"
    );
    let extension = if mime == "image/png" { "png" } else { "mp4" };
    let name = format!("{}.{}", mstudio::media::id(), extension);
    let object = format!("{}/{}", field("upload_dir")?.trim_end_matches('/'), name);
    let boundary = format!("mstudio-{}", mstudio::media::id());
    let fields: [(&str, Value); 7] = [
        ("OSSAccessKeyId", data["oss_access_key_id"].clone()),
        ("Signature", data["signature"].clone()),
        ("policy", data["policy"].clone()),
        ("x-oss-object-acl", data["x_oss_object_acl"].clone()),
        (
            "x-oss-forbid-overwrite",
            data["x_oss_forbid_overwrite"].clone(),
        ),
        ("key", Value::String(object.clone())),
        ("success_action_status", Value::String("200".into())),
    ];
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{}\r\n",
                value.as_str().context("上传凭证字段无效")?
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: {mime}\r\n\r\n").as_bytes());
    body.extend_from_slice(&bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    // Only the temporary OSS policy goes to storage; never forward the API key.
    let sent = client
        .post(host)
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await?;
    if !sent.status().is_success() {
        return Err(
            crate::app_error::http_error(sent, "reference_upload_transfer")
                .await
                .into(),
        );
    }
    Ok(format!("oss://{object}"))
}
