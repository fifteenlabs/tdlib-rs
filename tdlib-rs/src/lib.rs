// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
pub mod build;
#[cfg(not(feature = "build-only"))]
mod generated;
#[cfg(not(feature = "build-only"))]
mod observer;
#[cfg(not(feature = "build-only"))]
mod tdjson;

#[cfg(not(feature = "build-only"))]
pub use generated::{enums, functions, types};

/// Type alias for string types in generated code.
/// When the `gpui` feature is enabled, this resolves to `gpui::SharedString`.
/// Otherwise, it resolves to `String`.
#[cfg(all(feature = "gpui", not(feature = "build-only")))]
pub type TdString = gpui::SharedString;

#[cfg(all(not(feature = "gpui"), not(feature = "build-only")))]
pub type TdString = String;

/// Error type for TDLib function calls.
///
/// Wraps both TDLib API errors and deserialization failures so that
/// callers never see a panic from malformed responses.
#[cfg(not(feature = "build-only"))]
#[derive(Debug)]
pub enum TdError {
    /// A standard TDLib API error (e.g. 404, 429, etc.).
    Api(types::Error),
    /// The JSON response could not be deserialized into the expected Rust type.
    Deserialization {
        /// The Rust type we attempted to deserialize into (e.g. "Chat").
        expected_type: &'static str,
        /// The raw JSON payload that failed to deserialize.
        payload: String,
        /// The serde error.
        error: serde_json::Error,
    },
}

#[cfg(not(feature = "build-only"))]
impl std::fmt::Display for TdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TdError::Api(e) => write!(f, "TDLib error {}: {}", e.code, e.message),
            TdError::Deserialization {
                expected_type,
                error,
                ..
            } => write!(f, "Failed to deserialize {expected_type}: {error}"),
        }
    }
}

#[cfg(not(feature = "build-only"))]
impl std::error::Error for TdError {}

#[cfg(not(feature = "build-only"))]
impl TdError {
    /// Returns the API error code, or -1 for deserialization errors.
    pub fn code(&self) -> i32 {
        match self {
            TdError::Api(e) => e.code,
            TdError::Deserialization { .. } => -1,
        }
    }
}

#[cfg(not(feature = "build-only"))]
use enums::Update;
#[cfg(not(feature = "build-only"))]
use once_cell::sync::Lazy;
#[cfg(not(feature = "build-only"))]
use serde_json::Value;
#[cfg(not(feature = "build-only"))]
use std::sync::atomic::{AtomicU32, Ordering};

#[cfg(not(feature = "build-only"))]
static EXTRA_COUNTER: AtomicU32 = AtomicU32::new(0);
#[cfg(not(feature = "build-only"))]
static OBSERVER: Lazy<observer::Observer> = Lazy::new(observer::Observer::new);

/// Create a TdLib client returning its id. Note that to start receiving
/// updates for a client you need to send at least a request with it first.
#[cfg(not(feature = "build-only"))]
pub fn create_client() -> i32 {
    tdjson::create_client()
}

/// Receive a single update or response from TdLib. If it's an update, it
/// returns a tuple with the `Update` and the associated `client_id`.
/// Note that to start receiving updates for a client you need to send
/// at least a request with it first.
#[cfg(not(feature = "build-only"))]
pub fn receive() -> Option<(Update, i32)> {
    receive_with_timeout(std::time::Duration::from_secs(2))
}

/// Like [`receive`], but with a caller-specified timeout for `td_receive`.
#[cfg(not(feature = "build-only"))]
pub fn receive_with_timeout(timeout: std::time::Duration) -> Option<(Update, i32)> {
    let response = tdjson::receive(timeout.as_secs_f64());
    if let Some(response_str) = response {
        let response: Value = serde_json::from_str(&response_str).unwrap();

        match response.get("@extra") {
            Some(extra) => {
                let extra = extra.as_u64().unwrap() as u32;
                OBSERVER.notify(extra, response_str);
            }
            None => {
                let client_id = response["@client_id"].as_i64().unwrap() as i32;
                match serde_json::from_value(response) {
                    Ok(update) => {
                        return Some((update, client_id));
                    }
                    Err(e) => {
                        log::warn!("Received an unknown response: {response_str}\nReason: {e}");
                    }
                }
            }
        }
    }

    None
}

#[cfg(not(feature = "build-only"))]
pub(crate) async fn send_request(client_id: i32, mut request: Value) -> String {
    let extra = EXTRA_COUNTER.fetch_add(1, Ordering::Relaxed);
    request["@extra"] = serde_json::to_value(extra).unwrap();

    let receiver = OBSERVER.subscribe(extra);
    tdjson::send(client_id, request.to_string());

    receiver.await.unwrap()
}
