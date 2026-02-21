//! Claude API Integration for LLM-Assisted Tuning
//!
//! This module provides integration with the Anthropic Claude API
//! for suggesting parameter adjustments based on DNA analysis.

use harmonium_core::{dna::GlobalMetrics, tuning::TuningParams};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during Claude API communication
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Response parse error: {0}")]
    ParseError(String),

    #[error("API key not configured")]
    NoApiKey,

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
}

/// Claude API request structure
#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Claude API response structure
#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    #[allow(dead_code)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

/// Claude agent for LLM-assisted tuning
#[derive(Clone, Debug)]
pub struct ClaudeAgent {
    api_key: Option<String>,
    model: String,
    client: reqwest::Client,
}

impl Default for ClaudeAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeAgent {
    /// Create a new Claude agent
    #[must_use]
    pub fn new() -> Self {
        Self {
            api_key: None,
            model: "claude-sonnet-4-20250514".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Set the API key
    #[must_use]
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the model to use
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Check if the agent is configured with an API key
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    /// Request tuning suggestions from Claude (async)
    ///
    /// # Errors
    /// Returns error if API key is not configured or request fails
    pub async fn suggest_tuning(
        &self,
        reference: &GlobalMetrics,
        generated: &GlobalMetrics,
        current_tuning: &TuningParams,
    ) -> Result<TuningSuggestion, AgentError> {
        let api_key = self.api_key.as_ref().ok_or(AgentError::NoApiKey)?;

        // Build the prompt
        let prompt = self.build_prompt(reference, generated, current_tuning);

        // Make the API request
        let request_body = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 2048,
            messages: vec![Message { role: "user".to_string(), content: prompt }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        // Check for rate limiting
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AgentError::RateLimited);
        }

        // Check for other errors
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AgentError::RequestFailed(format!("HTTP {}: {}", status, body)));
        }

        // Parse response
        let claude_response: ClaudeResponse =
            response.json().await.map_err(|e| AgentError::ParseError(e.to_string()))?;

        // Extract text from response
        let response_text: String = claude_response
            .content
            .iter()
            .filter(|c| c.content_type == "text")
            .filter_map(|c| c.text.as_ref())
            .cloned()
            .collect();

        // Parse the JSON response from Claude
        self.parse_suggestion(&response_text, current_tuning)
    }

    /// Request tuning suggestions from Claude (blocking)
    ///
    /// # Errors
    /// Returns error if API key is not configured or request fails
    pub fn suggest_tuning_blocking(
        &self,
        reference: &GlobalMetrics,
        generated: &GlobalMetrics,
        current_tuning: &TuningParams,
    ) -> Result<TuningSuggestion, AgentError> {
        let api_key = self.api_key.as_ref().ok_or(AgentError::NoApiKey)?;

        // Build the prompt
        let prompt = self.build_prompt(reference, generated, current_tuning);

        // Make the API request using blocking client
        let client = reqwest::blocking::Client::new();

        let request_body = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 2048,
            messages: vec![Message { role: "user".to_string(), content: prompt }],
        };

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .map_err(|e| AgentError::RequestFailed(e.to_string()))?;

        // Check for rate limiting
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AgentError::RateLimited);
        }

        // Check for other errors
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(AgentError::RequestFailed(format!("HTTP {}: {}", status, body)));
        }

        // Parse response
        let claude_response: ClaudeResponse =
            response.json().map_err(|e| AgentError::ParseError(e.to_string()))?;

        // Extract text from response
        let response_text: String = claude_response
            .content
            .iter()
            .filter(|c| c.content_type == "text")
            .filter_map(|c| c.text.as_ref())
            .cloned()
            .collect();

        // Parse the JSON response from Claude
        self.parse_suggestion(&response_text, current_tuning)
    }

    /// Build the prompt for Claude
    fn build_prompt(
        &self,
        reference: &GlobalMetrics,
        generated: &GlobalMetrics,
        current_tuning: &TuningParams,
    ) -> String {
        format!(
            r#"You are a music theory expert tuning a generative music algorithm.

## Target Style Profile (from real music corpus)
- avg_voice_leading_effort: {:.2}
- tension_variance: {:.4}
- tension_release_balance: {:.2}
- diatonic_percentage: {:.1}%
- harmonic_rhythm: {:.2} chords/measure

## Generated Music Profile
- avg_voice_leading_effort: {:.2} ({})
- tension_variance: {:.4} ({})
- tension_release_balance: {:.2} ({})
- diatonic_percentage: {:.1}% ({})
- harmonic_rhythm: {:.2} ({})

## Current Tuning Parameters (with explanations)
```toml
# Harmony - Voice Leading
max_semitone_movement = {}  # Max semitones a voice can move (1-3). Lower = smoother voice leading.
cardinality_morph_enabled = {}  # Allow triad ↔ tetrad transitions
trq_threshold = {:.2}  # Tension/Release Quotient threshold for neighbor selection (0.0-1.0)

# Harmony - Strategy Selection (Steedman vs Neo-Riemannian)
steedman_lower_threshold = {:.2}  # Stay in Steedman while tension > this
steedman_upper_threshold = {:.2}  # Enter Steedman when tension < this
neo_riemannian_lower_threshold = {:.2}  # Enter Neo-Riemannian when tension > this
neo_riemannian_upper_threshold = {:.2}  # Stay in Neo-Riemannian while tension < this
hysteresis_boost = {:.2}  # Stability boost for current strategy (0.0-0.3)

# Rhythm - Perfect Balance Polygons
kick_density_threshold = {:.2}  # Density below this uses low-density kick pattern
kick_low_density_vertices = {}  # Kick polygon vertices at low density (2 = half notes)
kick_high_density_vertices = {}  # Kick polygon vertices at high density (4 = quarter notes)
snare_density_threshold = {:.2}  # Density threshold for snare pattern
snare_low_density_vertices = {}  # Snare vertices at low density
snare_high_density_vertices = {}  # Snare vertices at high density
hat_very_low_density_vertices = {}  # Hi-hat vertices at very low density
hat_low_density_vertices = {}  # Hi-hat vertices at low density
hat_medium_density_vertices = {}  # Hi-hat vertices at medium density
hat_high_density_vertices = {}  # Hi-hat vertices at high density
```

## Your Task
Analyze the divergence between generated and target profiles.
Suggest parameter adjustments to move generated closer to target.

Key relationships:
- Higher max_semitone_movement → higher avg_voice_leading_effort
- Lower trq_threshold → more stable/predictable harmony
- More kick/snare vertices → faster harmonic rhythm
- Lower steedman thresholds → more functional harmony (lower tension variance)

Return ONLY valid JSON (no markdown code blocks) in this exact format:
{{"reasoning": "Your analysis of what needs to change and why", "changes": {{"parameter_name": new_value, "another_param": new_value}}, "confidence": 0.8}}

Example response:
{{"reasoning": "Voice leading effort is too high (3.2 vs target 1.5). Reducing max_semitone_movement from 2 to 1 will create smoother voice leading.", "changes": {{"max_semitone_movement": 1}}, "confidence": 0.85}}"#,
            // Reference metrics
            reference.average_voice_leading_effort,
            reference.tension_variance,
            reference.tension_release_balance,
            reference.diatonic_percentage,
            reference.harmonic_rhythm,
            // Generated metrics with direction indicators
            generated.average_voice_leading_effort,
            Self::direction_indicator(
                generated.average_voice_leading_effort,
                reference.average_voice_leading_effort
            ),
            generated.tension_variance,
            Self::direction_indicator(generated.tension_variance, reference.tension_variance),
            generated.tension_release_balance,
            Self::direction_indicator(
                generated.tension_release_balance,
                reference.tension_release_balance
            ),
            generated.diatonic_percentage,
            Self::direction_indicator(generated.diatonic_percentage, reference.diatonic_percentage),
            generated.harmonic_rhythm,
            Self::direction_indicator(generated.harmonic_rhythm, reference.harmonic_rhythm),
            // Current tuning parameters
            current_tuning.max_semitone_movement,
            current_tuning.cardinality_morph_enabled,
            current_tuning.trq_threshold,
            current_tuning.steedman_lower_threshold,
            current_tuning.steedman_upper_threshold,
            current_tuning.neo_riemannian_lower_threshold,
            current_tuning.neo_riemannian_upper_threshold,
            current_tuning.hysteresis_boost,
            current_tuning.kick_density_threshold,
            current_tuning.kick_low_density_vertices,
            current_tuning.kick_high_density_vertices,
            current_tuning.snare_density_threshold,
            current_tuning.snare_low_density_vertices,
            current_tuning.snare_high_density_vertices,
            current_tuning.hat_very_low_density_vertices,
            current_tuning.hat_low_density_vertices,
            current_tuning.hat_medium_density_vertices,
            current_tuning.hat_high_density_vertices,
        )
    }

    /// Parse the suggestion from Claude's response
    fn parse_suggestion(
        &self,
        response_text: &str,
        current_tuning: &TuningParams,
    ) -> Result<TuningSuggestion, AgentError> {
        // Try to find JSON in the response (Claude might add text around it)
        let json_start = response_text.find('{');
        let json_end = response_text.rfind('}');

        let json_str = match (json_start, json_end) {
            (Some(start), Some(end)) if end > start => &response_text[start..=end],
            _ => {
                return Err(AgentError::ParseError("No JSON object found in response".to_string()));
            }
        };

        // Parse the JSON
        let raw: RawSuggestion = serde_json::from_str(json_str)
            .map_err(|e| AgentError::ParseError(format!("JSON parse error: {}", e)))?;

        // Convert changes to ParameterChange structs
        let parameter_changes = self.extract_parameter_changes(&raw.changes, current_tuning);

        Ok(TuningSuggestion {
            reasoning: raw.reasoning,
            parameter_changes,
            confidence: raw.confidence.unwrap_or(0.5),
        })
    }

    /// Extract parameter changes from the raw JSON
    fn extract_parameter_changes(
        &self,
        changes: &serde_json::Value,
        current: &TuningParams,
    ) -> Vec<ParameterChange> {
        let mut result = Vec::new();

        if let Some(obj) = changes.as_object() {
            for (key, value) in obj {
                let current_value = self.get_current_value(key, current);
                let suggested_value = format_json_value(value);

                if current_value != suggested_value {
                    result.push(ParameterChange {
                        name: key.clone(),
                        current: current_value,
                        suggested: suggested_value,
                        reason: String::new(), // Claude provides overall reasoning
                    });
                }
            }
        }

        result
    }

    /// Get the current value of a parameter as a string
    fn get_current_value(&self, name: &str, tuning: &TuningParams) -> String {
        match name {
            "max_semitone_movement" => tuning.max_semitone_movement.to_string(),
            "cardinality_morph_enabled" => tuning.cardinality_morph_enabled.to_string(),
            "trq_threshold" => format!("{:.2}", tuning.trq_threshold),
            "steedman_lower_threshold" => format!("{:.2}", tuning.steedman_lower_threshold),
            "steedman_upper_threshold" => format!("{:.2}", tuning.steedman_upper_threshold),
            "neo_riemannian_lower_threshold" => {
                format!("{:.2}", tuning.neo_riemannian_lower_threshold)
            }
            "neo_riemannian_upper_threshold" => {
                format!("{:.2}", tuning.neo_riemannian_upper_threshold)
            }
            "hysteresis_boost" => format!("{:.2}", tuning.hysteresis_boost),
            "kick_density_threshold" => format!("{:.2}", tuning.kick_density_threshold),
            "kick_low_density_vertices" => tuning.kick_low_density_vertices.to_string(),
            "kick_high_density_vertices" => tuning.kick_high_density_vertices.to_string(),
            "snare_density_threshold" => format!("{:.2}", tuning.snare_density_threshold),
            "snare_low_density_vertices" => tuning.snare_low_density_vertices.to_string(),
            "snare_high_density_vertices" => tuning.snare_high_density_vertices.to_string(),
            "hat_very_low_density_vertices" => tuning.hat_very_low_density_vertices.to_string(),
            "hat_low_density_vertices" => tuning.hat_low_density_vertices.to_string(),
            "hat_medium_density_vertices" => tuning.hat_medium_density_vertices.to_string(),
            "hat_high_density_vertices" => tuning.hat_high_density_vertices.to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Generate a direction indicator (up/down/equal)
    fn direction_indicator(generated: f32, reference: f32) -> &'static str {
        let diff = generated - reference;
        if diff.abs() < 0.01 {
            "="
        } else if diff > 0.0 {
            "↑ too high"
        } else {
            "↓ too low"
        }
    }
}

/// Raw suggestion from Claude (before processing)
#[derive(Deserialize)]
struct RawSuggestion {
    reasoning: String,
    changes: serde_json::Value,
    confidence: Option<f32>,
}

/// Format a JSON value as a string
fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                format!("{:.2}", f)
            } else {
                n.to_string()
            }
        }
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

/// A tuning suggestion from Claude
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TuningSuggestion {
    /// Reasoning behind the suggestion
    pub reasoning: String,

    /// Suggested parameter changes
    pub parameter_changes: Vec<ParameterChange>,

    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
}

impl TuningSuggestion {
    /// Apply this suggestion to a TuningParams, returning a new modified copy
    #[must_use]
    pub fn apply_to(&self, tuning: &TuningParams) -> TuningParams {
        let mut new_tuning = tuning.clone();

        for change in &self.parameter_changes {
            self.apply_change(&mut new_tuning, change);
        }

        new_tuning
    }

    /// Apply a single parameter change
    fn apply_change(&self, tuning: &mut TuningParams, change: &ParameterChange) {
        match change.name.as_str() {
            "max_semitone_movement" => {
                if let Ok(v) = change.suggested.parse::<u8>() {
                    tuning.max_semitone_movement = v;
                }
            }
            "cardinality_morph_enabled" => {
                if let Ok(v) = change.suggested.parse::<bool>() {
                    tuning.cardinality_morph_enabled = v;
                }
            }
            "trq_threshold" => {
                if let Ok(v) = change.suggested.parse::<f32>() {
                    tuning.trq_threshold = v;
                }
            }
            "steedman_lower_threshold" => {
                if let Ok(v) = change.suggested.parse::<f32>() {
                    tuning.steedman_lower_threshold = v;
                }
            }
            "steedman_upper_threshold" => {
                if let Ok(v) = change.suggested.parse::<f32>() {
                    tuning.steedman_upper_threshold = v;
                }
            }
            "neo_riemannian_lower_threshold" => {
                if let Ok(v) = change.suggested.parse::<f32>() {
                    tuning.neo_riemannian_lower_threshold = v;
                }
            }
            "neo_riemannian_upper_threshold" => {
                if let Ok(v) = change.suggested.parse::<f32>() {
                    tuning.neo_riemannian_upper_threshold = v;
                }
            }
            "hysteresis_boost" => {
                if let Ok(v) = change.suggested.parse::<f32>() {
                    tuning.hysteresis_boost = v;
                }
            }
            "kick_density_threshold" => {
                if let Ok(v) = change.suggested.parse::<f32>() {
                    tuning.kick_density_threshold = v;
                }
            }
            "kick_low_density_vertices" => {
                if let Ok(v) = change.suggested.parse::<usize>() {
                    tuning.kick_low_density_vertices = v;
                }
            }
            "kick_high_density_vertices" => {
                if let Ok(v) = change.suggested.parse::<usize>() {
                    tuning.kick_high_density_vertices = v;
                }
            }
            "snare_density_threshold" => {
                if let Ok(v) = change.suggested.parse::<f32>() {
                    tuning.snare_density_threshold = v;
                }
            }
            "snare_low_density_vertices" => {
                if let Ok(v) = change.suggested.parse::<usize>() {
                    tuning.snare_low_density_vertices = v;
                }
            }
            "snare_high_density_vertices" => {
                if let Ok(v) = change.suggested.parse::<usize>() {
                    tuning.snare_high_density_vertices = v;
                }
            }
            "hat_very_low_density_vertices" => {
                if let Ok(v) = change.suggested.parse::<usize>() {
                    tuning.hat_very_low_density_vertices = v;
                }
            }
            "hat_low_density_vertices" => {
                if let Ok(v) = change.suggested.parse::<usize>() {
                    tuning.hat_low_density_vertices = v;
                }
            }
            "hat_medium_density_vertices" => {
                if let Ok(v) = change.suggested.parse::<usize>() {
                    tuning.hat_medium_density_vertices = v;
                }
            }
            "hat_high_density_vertices" => {
                if let Ok(v) = change.suggested.parse::<usize>() {
                    tuning.hat_high_density_vertices = v;
                }
            }
            _ => {} // Unknown parameter, ignore
        }
    }

    /// Check if this suggestion has any changes
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.parameter_changes.is_empty()
    }
}

/// A single parameter change suggestion
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParameterChange {
    /// Parameter name
    pub name: String,

    /// Current value (as string for display)
    pub current: String,

    /// Suggested new value (as string for display)
    pub suggested: String,

    /// Reason for this specific change
    pub reason: String,
}

impl std::fmt::Display for ParameterChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} → {}", self.name, self.current, self.suggested)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = ClaudeAgent::new();
        assert!(!agent.is_configured());
    }

    #[test]
    fn test_agent_with_key() {
        let agent = ClaudeAgent::new().with_api_key("test-key");
        assert!(agent.is_configured());
    }

    #[test]
    fn test_direction_indicator() {
        assert_eq!(ClaudeAgent::direction_indicator(2.0, 1.0), "↑ too high");
        assert_eq!(ClaudeAgent::direction_indicator(1.0, 2.0), "↓ too low");
        assert_eq!(ClaudeAgent::direction_indicator(1.0, 1.005), "=");
    }

    #[test]
    fn test_prompt_generation() {
        let agent = ClaudeAgent::new();
        let reference = GlobalMetrics::default();
        let generated = GlobalMetrics::default();
        let tuning = TuningParams::default();

        let prompt = agent.build_prompt(&reference, &generated, &tuning);

        assert!(prompt.contains("Target Style Profile"));
        assert!(prompt.contains("Generated Music Profile"));
        assert!(prompt.contains("Current Tuning Parameters"));
        assert!(prompt.contains("max_semitone_movement"));
    }

    #[test]
    fn test_parse_suggestion() {
        let agent = ClaudeAgent::new();
        let tuning = TuningParams::default();

        let response = r#"{"reasoning": "Voice leading is too high", "changes": {"max_semitone_movement": 1}, "confidence": 0.8}"#;

        let suggestion = agent.parse_suggestion(response, &tuning).unwrap();

        assert_eq!(suggestion.confidence, 0.8);
        assert!(suggestion.reasoning.contains("Voice leading"));
        assert!(suggestion.has_changes());
    }

    #[test]
    fn test_apply_suggestion() {
        let tuning = TuningParams::default();
        let suggestion = TuningSuggestion {
            reasoning: "Test".to_string(),
            parameter_changes: vec![ParameterChange {
                name: "max_semitone_movement".to_string(),
                current: "2".to_string(),
                suggested: "1".to_string(),
                reason: "".to_string(),
            }],
            confidence: 0.8,
        };

        let new_tuning = suggestion.apply_to(&tuning);
        assert_eq!(new_tuning.max_semitone_movement, 1);
    }
}
