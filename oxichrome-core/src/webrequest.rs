//! Observe-only `chrome.webRequest.onCompleted` binding.
//!
//! This module does **not** support blocking, request/response modification,
//! or response-body access. It only delivers completed-request metadata so
//! the extension background can react (e.g. trigger a separate acquisition).

use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

use crate::error::Result;
use crate::js_bridge;

/// Chrome RequestFilter subset needed for URL observation.
/// Only `urls` is exposed — sufficient for matching completed request URLs.
#[derive(Debug, Clone)]
pub struct RequestFilter {
    pub urls: Vec<String>,
}

impl RequestFilter {
    pub fn urls(urls: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            urls: urls.into_iter().map(Into::into).collect(),
        }
    }

    fn to_js(&self) -> Result<JsValue> {
        let obj = js_sys::Object::new();
        let arr = js_sys::Array::new();
        for u in &self.urls {
            arr.push(&JsValue::from_str(u));
        }
        js_sys::Reflect::set(&obj, &JsValue::from_str("urls"), &arr)
            .map_err(crate::error::OxichromeError::from)?;
        Ok(obj.into())
    }
}

/// Subset of Chrome's `webRequest.OnCompletedDetails` required by callers.
/// Field names follow Chrome's camelCase via serde rename for deserialization
/// from the JS event payload; Rust fields use snake_case.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletedDetails {
    pub url: String,
    pub method: String,
    /// HTTP status code of the completed response (`statusCode` in Chrome).
    pub status_code: u16,
}

impl CompletedDetails {
    fn from_js(value: JsValue) -> Result<Self> {
        Ok(serde_wasm_bindgen::from_value(value)?)
    }
}

/// Register an observe-only `onCompleted` listener.
///
/// - `filter` limits which requests produce events (typically URL patterns).
/// - `callback` receives typed details; it may schedule async work via
///   `wasm_bindgen_futures::spawn_local` (same pattern as `#[oxichrome::on]`).
///
/// The underlying Chrome API is invoked **without** `extraInfoSpec` and
/// without blocking permission — the listener cannot cancel or modify the
/// request.
pub fn on_completed<F>(filter: &RequestFilter, mut callback: F) -> Result<()>
where
    F: FnMut(CompletedDetails) + 'static,
{
    let filter_js = filter.to_js()?;

    let closure = Closure::wrap(Box::new(move |details: JsValue| {
        match CompletedDetails::from_js(details) {
            Ok(d) => callback(d),
            Err(e) => {
                web_sys::console::warn_1(&JsValue::from_str(&format!(
                    "oxichrome::webrequest: failed to parse onCompleted details: {e:?}"
                )));
            }
        }
    }) as Box<dyn FnMut(JsValue)>);

    js_bridge::chrome_web_request_on_completed_add_listener(&closure, &filter_js);
    // Leak the closure so it lives for the service-worker lifetime (same
    // pattern as oxichrome event-handler macro's `closure.forget()`).
    closure.forget();
    Ok(())
}
