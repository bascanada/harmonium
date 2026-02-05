//! VST GUI Module - Webview-based editor for the Harmonium plugin
//!
//! This module provides a webview-based GUI for the VST plugin using nih-plug-webview.
//! The UI is built with Svelte and communicates with the plugin via JSON messages.

#[cfg(feature = "vst-gui")]
mod message_handler;
#[cfg(feature = "vst-gui")]
mod state_serializer;
#[cfg(feature = "vst-gui")]
mod webview_editor;

#[cfg(feature = "vst-gui")]
pub use webview_editor::create_editor;
