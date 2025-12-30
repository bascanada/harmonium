use wasm_bindgen::prelude::*;

pub mod sequencer;
pub mod harmony;
pub mod log;
pub mod engine;
pub mod audio;

#[wasm_bindgen]
pub struct Handle {
    #[allow(dead_code)]
    stream: cpal::Stream,
    bpm: f32,
    key: String,
    scale: String,
    pulses: usize,
    steps: usize,
}

#[wasm_bindgen]
impl Handle {
    pub fn get_bpm(&self) -> f32 {
        self.bpm
    }

    pub fn get_key(&self) -> String {
        self.key.clone()
    }

    pub fn get_scale(&self) -> String {
        self.scale.clone()
    }

    pub fn get_pulses(&self) -> usize {
        self.pulses
    }

    pub fn get_steps(&self) -> usize {
        self.steps
    }
}

#[wasm_bindgen]
pub fn start() -> Result<Handle, JsValue> {
    console_error_panic_hook::set_once();

    let (stream, config) = audio::create_stream().map_err(|e| JsValue::from_str(&e))?;

    Ok(Handle { 
        stream,
        bpm: config.bpm,
        key: config.key,
        scale: config.scale,
        pulses: config.pulses,
        steps: config.steps,
    })
}

