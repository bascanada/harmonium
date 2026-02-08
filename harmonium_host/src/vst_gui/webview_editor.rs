//! Webview Editor - Creates a webview-based GUI for the VST plugin

use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};

use harmonium_core::params::ControlMode;
use nih_plug::prelude::*;
use nih_plug_webview::{HTMLSource, WebViewEditor, http};
use serde_json::Value;

use super::{
    message_handler::handle_message,
    state_serializer::{collect_state, create_state_update_message},
};
use crate::{engine::EngineParams, vst_plugin::HarmoniumParams};

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
static HTML_CONTENT: &[u8] = include_bytes!("../../../web/dist/vst/index.html");
static JS_CONTENT: &[u8] = include_bytes!("../../../web/dist/vst/index.js");
static CSS_CONTENT: &[u8] = include_bytes!("../../../web/dist/vst/index.css");
static NOT_FOUND: &[u8] = b"Not found";

/// Create the webview editor
pub fn create_editor(
    target_state: Arc<Mutex<EngineParams>>,
    control_mode: Arc<Mutex<ControlMode>>,
    params: Arc<HarmoniumParams>,
    harmony_state_rx: Arc<Mutex<Option<rtrb::Consumer<crate::engine::HarmonyState>>>>,
) -> Option<Box<dyn Editor>> {
    // Clone for the event loop closure
    let target_state_clone = target_state.clone();
    let control_mode_clone = control_mode.clone();
    let params_clone = params.clone();
    let harmony_state_rx_clone = harmony_state_rx.clone();

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

        // Consume harmony state updates from the engine's lock-free queue
        // and update ControlMode for UI visualization
        if let Ok(mut rx_opt) = harmony_state_rx_clone.lock() {
            if let Some(rx) = rx_opt.as_mut() {
                // Drain all available states and keep only the latest
                let mut latest: Option<crate::engine::HarmonyState> = None;
                while let Ok(state) = rx.pop() {
                    latest = Some(state);
                }
                // Update control_mode with latest harmony state
                if let Some(harmony_state) = latest {
                    // Log receipt of state
                    if !harmony_state.look_ahead_buffer.is_empty() {
                         nih_plug::nih_log!("VSGUI: Received HarmonyState with buffer size: {}", harmony_state.look_ahead_buffer.len());
                    }

                    if let Ok(mut mode) = control_mode_clone.lock() {
                        mode.current_step = harmony_state.current_step as u32;
                        mode.current_measure = harmony_state.measure_number as u32;
                        mode.current_chord = harmony_state.chord_name.to_string();
                        mode.is_minor_chord = harmony_state.chord_is_minor;
                        mode.progression_name = harmony_state.progression_name.to_string();
                        // Convert fixed-size arrays to Vec for UI
                        let primary_len = harmony_state.primary_steps.min(192);
                        mode.primary_pattern =
                            harmony_state.primary_pattern[..primary_len].to_vec();
                        let secondary_len = harmony_state.secondary_steps.min(192);
                        mode.secondary_pattern =
                            harmony_state.secondary_pattern[..secondary_len].to_vec();
                        mode.look_ahead_buffer = harmony_state.look_ahead_buffer.clone();
                    }
                }
            }
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
