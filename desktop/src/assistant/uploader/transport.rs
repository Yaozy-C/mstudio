//! Project file IDs only at the HTTP boundary, after the SDK encoded each API.
use super::Uploader;
use bytes::Bytes;
use rig_core::{
    http_client::{
        HttpClientExt, LazyBody, MultipartForm, Request, Response, Result, StreamingResponse,
    },
    wasm_compat::WasmCompatSend,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct FileTransport {
    inner: rig_reqwest::ReqwestClient,
    pub(super) files: Arc<Uploader>,
}
impl FileTransport {
    pub fn new(inner: rig_reqwest::ReqwestClient, files: Arc<Uploader>) -> Self {
        Self { inner, files }
    }

    fn handles(&self, request: &Request<Bytes>) -> bool {
        let Ok(root) = reqwest::Url::parse(&self.files.root) else {
            return false;
        };
        request.method() == "POST"
            && request.uri().authority().map(|v| v.as_str()) == root.as_str().split('/').nth(2)
            && self.files.backend.handles(request.uri().path())
    }

    fn projected(&self, template: &Request<Bytes>, body: Vec<u8>) -> Request<Bytes> {
        let mut request = Request::new(Bytes::from(body));
        *request.method_mut() = template.method().clone();
        *request.uri_mut() = template.uri().clone();
        *request.headers_mut() = template.headers().clone();
        *request.version_mut() = template.version();
        *request.extensions_mut() = template.extensions().clone();
        request.headers_mut().remove("content-length");
        self.files
            .backend
            .headers(template.uri().path(), request.headers_mut());
        request
    }
}

impl HttpClientExt for FileTransport {
    fn send<T, U>(
        &self,
        request: Request<T>,
    ) -> impl Future<Output = Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        T: Into<Bytes> + WasmCompatSend,
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        let this = self.clone();
        let original = request.map(Into::into);
        async move {
            if !this.handles(&original) {
                return this.inner.send(original).await;
            }
            for attempt in 0..2 {
                let (body, used) = this.files.prepare(original.body()).await?;
                match this.inner.send(this.projected(&original, body)).await {
                    Err(error) => {
                        let stale = this.files.invalidate_stale(&error, &used)?;
                        if attempt == 0 && stale {
                            continue;
                        }
                        return Err(error);
                    }
                    result => return result,
                }
            }
            unreachable!("a second rejected file ID is returned to the caller")
        }
    }

    fn send_streaming<T>(
        &self,
        request: Request<T>,
    ) -> impl Future<Output = Result<StreamingResponse>> + WasmCompatSend
    where
        T: Into<Bytes> + WasmCompatSend,
    {
        let original = request.map(Into::into);
        async move {
            if !self.handles(&original) {
                return self.inner.send_streaming(original).await;
            }
            for attempt in 0..2 {
                let (body, used) = self.files.prepare(original.body()).await?;
                match self
                    .inner
                    .send_streaming(self.projected(&original, body))
                    .await
                {
                    Err(error) => {
                        let stale = self.files.invalidate_stale(&error, &used)?;
                        if attempt == 0 && stale {
                            continue;
                        }
                        return Err(error);
                    }
                    result => return result,
                }
            }
            unreachable!("a second rejected file ID is returned to the caller")
        }
    }

    fn send_multipart<U>(
        &self,
        request: Request<MultipartForm>,
    ) -> impl Future<Output = Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        self.inner.send_multipart(request)
    }
}
