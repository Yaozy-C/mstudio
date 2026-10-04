//! Embedded pure domain code, built from the same TypeScript tested by the UI.
//! A fresh bounded runtime per call prevents state leaking between projects.
use anyhow::{Context as _, Result};
use rquickjs::{Context, Function, Runtime};
use serde_json::Value;
pub fn execute(input: Value) -> Result<Value> {
    let _total = crate::diagnostics::Timing::new("domain_total");
    let runtime = Runtime::new()?;
    runtime.set_memory_limit(128 * 1024 * 1024);
    runtime.set_max_stack_size(2 * 1024 * 1024);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    runtime.set_interrupt_handler(Some(Box::new(move || {
        std::time::Instant::now() >= deadline
    })));
    let context = Context::full(&runtime)?;
    let raw = context.with(|ctx| -> Result<String> {
        ctx.globals().set("domainUuid", Function::new(ctx.clone(), mstudio::media::id)?)?;
        // QuickJS does not provide the browser URL constructor. Parse with the
        // host URL library without exposing network or filesystem operations.
        ctx.globals().set("domainParseUrl", Function::new(ctx.clone(), |value: String| {
            reqwest::Url::parse(&value).ok().map(|url| serde_json::json!({
                "protocol": format!("{}:", url.scheme()),
                "username": url.username(),
                "password": url.password().unwrap_or(""),
                "hostname": url.host_str().unwrap_or(""),
                "hash": url.fragment().filter(|s| !s.is_empty()).map(|s| format!("#{s}")).unwrap_or_default(),
            })).unwrap_or(Value::Null).to_string()
        })?)?;
        ctx.eval::<(), _>(r#"
            globalThis.crypto = { randomUUID: domainUuid };
            globalThis.structuredClone = value => JSON.parse(JSON.stringify(value));
            globalThis.TextEncoder = class { encode(value) {
                return new Uint8Array(Array.from(unescape(encodeURIComponent(value)), c => c.charCodeAt(0)));
            }};
        "#)?;
        let initialization = crate::diagnostics::Timing::new("domain_initialize");
        ctx.eval::<(), _>(include_str!(concat!(env!("OUT_DIR"), "/domain.js")))
            .map_err(|e| anyhow::anyhow!("Domain initialization: {e}: {:?}", ctx.catch()))?;
        drop(initialization);
        let _execution = crate::diagnostics::Timing::new("domain_execute");
        let function: Function = ctx.globals().get("domainExecute")?;
        function.call((input.to_string(),)).context("Domain execution failed")
    })?;
    serde_json::from_str(&raw).context("Invalid domain response")
}
