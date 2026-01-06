//! Webview Editor - Creates a webview-based GUI for the VST plugin

use nih_plug::prelude::*;
use nih_plug_webview::{http, WebViewEditor, HTMLSource};
use serde_json::Value;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use crate::engine::EngineParams;
use crate::params::ControlMode;
use crate::vst_plugin::HarmoniumParams;

use super::message_handler::handle_message;
use super::state_serializer::{collect_state, create_state_update_message};

/// Extract message string from serde_json::Value
/// Handles both String values (pre-serialized JSON) and Object values
fn extract_message_string(value: &Value) -> String {
    match value {
        // If it's already a string, use it directly (it's pre-serialized JSON)
        Value::String(s) => s.clone(),
        // Otherwise serialize the value to JSON
        _ => value.to_string(),
    }
}

/// Size of the editor window
const EDITOR_WIDTH: u32 = 1130;
const EDITOR_HEIGHT: u32 = 990;

/// Embedded content for the webview (static lifetimes)
static HTML_CONTENT: &[u8] = include_bytes!("../../web/dist/vst/index.html");
static JS_CONTENT: &[u8] = include_bytes!("../../web/dist/vst/index.js");
static CSS_CONTENT: &[u8] = include_bytes!("../../web/dist/vst/index.css");
static NOT_FOUND: &[u8] = b"Not found";

/// Create the webview editor
pub fn create_editor(
    target_state: Arc<Mutex<EngineParams>>,
    control_mode: Arc<Mutex<ControlMode>>,
    params: Arc<HarmoniumParams>,
) -> Option<Box<dyn Editor>> {
    // Clone for the event loop closure
    let target_state_clone = target_state.clone();
    let control_mode_clone = control_mode.clone();
    let params_clone = params.clone();

    // Frame counter for throttling state updates
    let frame_counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let frame_counter_clone = frame_counter.clone();

    let editor = WebViewEditor::new(
        HTMLSource::URL("harmonium://localhost/index.html"),
        (EDITOR_WIDTH, EDITOR_HEIGHT),
    )
    .with_background_color((23, 23, 23, 255)) // #171717
    .with_developer_mode(true) // Enable devtools for debugging
    .with_custom_protocol("harmonium".to_string(), move |request| {
        let path = request.uri().path();

        // Serve static content based on path
        let (content, mime_type): (&'static [u8], &str) = match path {
            "/index.html" | "/" => (HTML_CONTENT, "text/html; charset=utf-8"),
            "/index.js" => (JS_CONTENT, "application/javascript; charset=utf-8"),
            "/index.css" => (CSS_CONTENT, "text/css; charset=utf-8"),
            _ => {
                return http::Response::builder()
                    .status(404)
                    .header("Content-Type", "text/plain")
                    .body(Cow::Borrowed(NOT_FOUND))
                    .map_err(Into::into);
            }
        };

        http::Response::builder()
            .status(200)
            .header("Content-Type", mime_type)
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Borrowed(content))
            .map_err(Into::into)
    })
    .with_event_loop(move |ctx, _setter, _window| {
        // Handle incoming messages from webview
        while let Ok(msg) = ctx.next_event() {
            let msg_str = extract_message_string(&msg);
            handle_message(&msg_str, &target_state_clone, &control_mode_clone, &params_clone);
        }

        // Throttle state updates to ~30Hz (every 2 frames at 60fps)
        let frame = frame_counter_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if frame % 2 == 0 {
            // Collect and send state update
            let state = collect_state(&target_state_clone, &control_mode_clone);
            let msg = create_state_update_message(&state);
            if let Ok(json) = serde_json::from_str(&msg) {
                ctx.send_json(json);
            }
        }
    });

    Some(Box::new(editor))
}
