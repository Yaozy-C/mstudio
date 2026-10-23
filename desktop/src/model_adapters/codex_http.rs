//! Codex's internal Images transport. No agent turn and no automatic POST retries.
use super::{codex_auth::Credentials, codex_request};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
#[path = "codex_http_errors.rs"]
mod errors;

pub(super) const BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
pub(super) fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(600))
        .user_agent("mstudio-codex-images/0.1")
        .build()?)
}
pub(super) async fn request(
    client: &reqwest::Client,
    base: &str,
    credentials: &Credentials,
    input: &Value,
) -> Result<Vec<Value>> {
    let (path, body) = codex_request::payload(input)?;
    let mut response = client
        .post(format!("{base}/{path}"))
        .bearer_auth(&credentials.token)
        .header("ChatGPT-Account-ID", &credentials.account)
        .header("originator", "mstudio")
        .header("x-codex-image-turn-id", mstudio::media::id())
        .json(&body)
        .send()
        .await
        .map_err(errors::transport)?;
    if !response.status().is_success() {
        return Err(errors::response(response).await.into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(errors::transport)? {
        ensure!(
            bytes.len() + chunk.len() <= 90_000_000,
            "Codex 图片响应超过大小限制"
        );
        bytes.extend_from_slice(&chunk);
    }
    let result: Value = serde_json::from_slice(&bytes).context("Codex 返回无效图片响应")?;
    codex_request::image_result(&result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    #[tokio::test]
    async fn posts_exact_prompt_and_edit_references_and_does_not_leak_errors() {
        for (status, edit) in [
            (200, false),
            (200, true),
            (401, false),
            (404, false),
            (302, false),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let (end, length) = loop {
                    let mut buf = [0; 4096];
                    let n = socket.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..i]).to_lowercase();
                        assert!(headers.contains("authorization: bearer fake-token"));
                        assert!(headers.contains("chatgpt-account-id: fake-account"));
                        let length: usize = headers
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length: "))
                            .unwrap()
                            .parse()
                            .unwrap();
                        break (i + 4, length);
                    }
                };
                while bytes.len() < end + length {
                    let mut buf = [0; 4096];
                    let n = socket.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                }
                let body: Value = serde_json::from_slice(&bytes[end..end + length]).unwrap();
                assert_eq!(body["model"], "gpt-image-2.5-sunburst");
                assert_eq!(body["prompt"], "  中文\n\"unchanged\"\t");
                assert_eq!(body["size"], "2048x1152");
                assert_eq!(body["quality"], "high");
                let line = String::from_utf8_lossy(&bytes[..end]);
                assert!(line.starts_with(if edit {
                    "POST /images/edits "
                } else {
                    "POST /images/generations "
                }));
                if edit {
                    assert_eq!(body["images"][0]["image_url"], "data:image/png;base64,YQ==");
                }
                let body = if status == 200 {
                    r#"{"data":[{"b64_json":"iVBORw0KGgo="}]}"#
                } else {
                    "fake-token private prompt"
                };
                write!(socket, "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nLocation: http://127.0.0.1:1/secret\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            let mut input =
                json!({"prompt":"  中文\n\"unchanged\"\t", "model":"gpt-image-2.5-sunburst"});
            if edit {
                input["image"] = json!(["data:image/png;base64,YQ=="]);
            }
            let result = request(
                &client().unwrap(),
                &base,
                &Credentials {
                    token: "fake-token".into(),
                    account: "fake-account".into(),
                },
                &input,
            )
            .await;
            if status == 200 {
                assert_eq!(result.unwrap().len(), 1);
            } else {
                let error = result.unwrap_err().to_string();
                let issue: crate::app_error::AppError = serde_json::from_str(&error).unwrap();
                assert_eq!(issue.http_status, Some(status));
                assert!(!error.contains("fake-token"));
                assert!(!error.contains("private prompt"));
                if status == 404 {
                    assert!(error.contains("模型 ID"));
                    assert!(error.contains("未切换型号"));
                }
            }
            server.join().unwrap();
        }
    }
}
