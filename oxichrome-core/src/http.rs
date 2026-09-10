use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestCredentials, RequestInit, RequestMode, Response};

use crate::error::Result;

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub async fn post_json(url: &str, body: &str) -> Result<HttpResponse> {
    let options = RequestInit::new();

    options.set_method("POST");
    options.set_mode(RequestMode::Cors);
    options.set_credentials(RequestCredentials::Include);
    options.set_body(&JsValue::from_str(body));

    let request = Request::new_with_str_and_init(url, &options)?;

    let headers: Headers = request.headers();
    headers.set("Content-Type", "application/json")?;

    let global = js_sys::global();

    let fetch_value = Reflect::get(&global, &JsValue::from_str("fetch"))?;
    let fetch: Function = fetch_value
        .dyn_into()
        .map_err(|_| JsValue::from_str("global fetch is unavailable"))?;

    let promise_value = fetch.call1(&global, &request)?;

    let promise: Promise = promise_value
        .dyn_into()
        .map_err(|_| JsValue::from_str("global fetch did not return a Promise"))?;

    let response_value = JsFuture::from(promise).await?;

    let response: Response = response_value.dyn_into()?;

    let status = response.status();

    let body_value = JsFuture::from(response.text()?).await?;

    let response_body = body_value.as_string().unwrap_or_default();

    Ok(HttpResponse {
        status,
        body: response_body,
    })
}
