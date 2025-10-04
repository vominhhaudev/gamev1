use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{warn, error};

/// Input validation errors
#[derive(Debug, Clone)]
pub enum ValidationError {
    InvalidPlayerId(String),
    InvalidSequence(u32),
    InvalidMovement(String),
    InvalidTimestamp(u64),
    SequenceTooOld(u32, u32),
    SequenceDuplicate(u32),
    TimestampTooOld(u64, u64),
    TimestampTooNew(u64, u64),
    RateLimitExceeded,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidPlayerId(msg) => write!(f, "Invalid player_id: {}", msg),
            ValidationError::InvalidSequence(seq) => write!(f, "Invalid sequence: {}", seq),
            ValidationError::InvalidMovement(msg) => write!(f, "Invalid movement: {}", msg),
            ValidationError::InvalidTimestamp(ts) => write!(f, "Invalid timestamp: {}", ts),
            ValidationError::SequenceTooOld(expected, actual) => write!(f, "Sequence too old: expected > {}, got {}", expected, actual),
            ValidationError::SequenceDuplicate(seq) => write!(f, "Duplicate sequence: {}", seq),
            ValidationError::TimestampTooOld(expected, actual) => write!(f, "Timestamp too old: expected > {}, got {}", expected, actual),
            ValidationError::TimestampTooNew(expected, actual) => write!(f, "Timestamp too new: expected < {}, got {}", expected, actual),
            ValidationError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
        }
    }
}

/// Validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum movement magnitude allowed
    pub max_movement_magnitude: f32,
    /// Maximum timestamp difference (ms)
    pub max_timestamp_diff_ms: u64,
    /// Maximum sequence gap allowed
    pub max_sequence_gap: u32,
    /// Rate limiting: max inputs per second per player
    pub max_inputs_per_second: u32,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_movement_magnitude: 10.0,
            max_timestamp_diff_ms: 10000, // 10 seconds
            max_sequence_gap: 100,
            max_inputs_per_second: 60, // 60 FPS max
        }
    }
}

/// Input validator for game inputs
pub struct InputValidator {
    config: ValidationConfig,
    /// Track last sequence per player
    last_sequences: HashMap<String, u32>,
    /// Track input timestamps for rate limiting
    input_timestamps: HashMap<String, Vec<u64>>,
}

impl InputValidator {
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            last_sequences: HashMap::new(),
            input_timestamps: HashMap::new(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(ValidationConfig::default())
    }

    /// Validate player input comprehensively
    pub fn validate_input(&mut self, input: &crate::simulation::PlayerInput) -> Result<(), ValidationError> {
        // Validate player_id
        self.validate_player_id(&input.player_id)?;

        // Validate movement vector
        self.validate_movement(&input.movement)?;

        // Validate timestamp
        self.validate_timestamp(input.timestamp)?;

        // Validate sequence number
        self.validate_sequence(&input.player_id, input.input_sequence)?;

        // Check rate limiting
        self.check_rate_limit(&input.player_id)?;

        // Update tracking data
        self.update_tracking(&input.player_id, input.input_sequence, input.timestamp);

        Ok(())
    }

    fn validate_player_id(&self, player_id: &str) -> Result<(), ValidationError> {
        if player_id.is_empty() {
            return Err(ValidationError::InvalidPlayerId("Empty player_id".to_string()));
        }

        if player_id.len() > 50 {
            return Err(ValidationError::InvalidPlayerId("Player_id too long".to_string()));
        }

        // Check for invalid characters (only allow alphanumeric, underscore, hyphen)
        if !player_id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(ValidationError::InvalidPlayerId("Invalid characters in player_id".to_string()));
        }

        Ok(())
    }

    fn validate_movement(&self, movement: &[f32; 3]) -> Result<(), ValidationError> {
        for (i, &val) in movement.iter().enumerate() {
            if val.is_nan() {
                return Err(ValidationError::InvalidMovement(format!("NaN value at index {}", i)));
            }

            if val.is_infinite() {
                return Err(ValidationError::InvalidMovement(format!("Infinite value at index {}", i)));
            }

            if val.abs() > self.config.max_movement_magnitude {
                return Err(ValidationError::InvalidMovement(format!(
                    "Movement magnitude too large at index {}: {} > {}",
                    i, val.abs(), self.config.max_movement_magnitude
                )));
            }
        }

        Ok(())
    }

    fn validate_timestamp(&self, timestamp: u64) -> Result<(), ValidationError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ValidationError::InvalidTimestamp(timestamp))?
            .as_millis() as u64;

        let diff = if now > timestamp {
            now - timestamp
        } else {
            timestamp - now
        };

        if diff > self.config.max_timestamp_diff_ms {
            return Err(ValidationError::TimestampTooOld(now, timestamp));
        }

        Ok(())
    }

    fn validate_sequence(&mut self, player_id: &str, sequence: u32) -> Result<(), ValidationError> {
        let last_sequence = self.last_sequences.get(player_id).copied().unwrap_or(0);

        // Check for duplicate sequence
        if sequence == last_sequence && last_sequence != 0 {
            return Err(ValidationError::SequenceDuplicate(sequence));
        }

        // Check for sequence gap (too old)
        if sequence < last_sequence && last_sequence - sequence > self.config.max_sequence_gap {
            return Err(ValidationError::SequenceTooOld(last_sequence, sequence));
        }

        Ok(())
    }

    fn check_rate_limit(&mut self, player_id: &str) -> Result<(), ValidationError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ValidationError::RateLimitExceeded)?
            .as_millis() as u64;

        let timestamps = self.input_timestamps.entry(player_id.to_string()).or_insert_with(Vec::new);

        // Remove old timestamps (older than 1 second)
        timestamps.retain(|&ts| now - ts < 1000);

        if timestamps.len() >= self.config.max_inputs_per_second as usize {
            return Err(ValidationError::RateLimitExceeded);
        }

        Ok(())
    }

    fn update_tracking(&mut self, player_id: &str, sequence: u32, timestamp: u64) {
        self.last_sequences.insert(player_id.to_string(), sequence);

        let timestamps = self.input_timestamps.entry(player_id.to_string()).or_insert_with(Vec::new);
        timestamps.push(timestamp);

        // Clean up old timestamps periodically
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        timestamps.retain(|&ts| now - ts < 2000); // Keep 2 seconds worth
    }

    /// Clean up old data periodically
    pub fn cleanup(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Remove old timestamp data
        self.input_timestamps.retain(|_, timestamps| {
            timestamps.retain(|&ts| now - ts < 2000);
            !timestamps.is_empty()
        });

        // Remove old sequence data (players inactive for > 5 minutes)
        self.last_sequences.retain(|_, _| {
            // For now, keep all sequences (could add timestamp tracking per player)
            true
        });
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// Utility functions for validation
pub mod utils {
    use super::*;

    /// Validate input JSON string before parsing
    pub fn validate_input_json(json_str: &str) -> Result<(), ValidationError> {
        if json_str.is_empty() {
            return Err(ValidationError::InvalidMovement("Empty input JSON".to_string()));
        }

        if json_str.len() > 4096 { // 4KB limit
            return Err(ValidationError::InvalidMovement("Input JSON too large".to_string()));
        }

        Ok(())
    }

    /// Parse and validate input in one step
    pub fn parse_and_validate_input(
        json_str: &str,
        validator: &mut InputValidator,
    ) -> Result<crate::simulation::PlayerInput, ValidationError> {
        // First validate JSON structure
        validate_input_json(json_str)?;

        // Parse JSON
        let input: crate::simulation::PlayerInput = serde_json::from_str(json_str)
            .map_err(|e| ValidationError::InvalidMovement(format!("JSON parse error: {}", e)))?;

        // Validate the parsed input
        validator.validate_input(&input)?;

        Ok(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_id_validation() {
        let validator = InputValidator::with_default_config();

        // Valid player IDs
        assert!(validator.validate_player_id("player123").is_ok());
        assert!(validator.validate_player_id("player_123").is_ok());
        assert!(validator.validate_player_id("player-123").is_ok());

        // Invalid player IDs
        assert!(validator.validate_player_id("").is_err());
        assert!(validator.validate_player_id("player@123").is_err());
        assert!(validator.validate_player_id(&"a".repeat(51)).is_err());
    }

    #[test]
    fn test_movement_validation() {
        let validator = InputValidator::with_default_config();

        // Valid movement
        assert!(validator.validate_movement(&[1.0, 0.0, 1.0]).is_ok());
        assert!(validator.validate_movement(&[0.0, 0.0, 0.0]).is_ok());

        // Invalid movement - NaN
        assert!(validator.validate_movement(&[f32::NAN, 0.0, 0.0]).is_err());

        // Invalid movement - Infinite
        assert!(validator.validate_movement(&[f32::INFINITY, 0.0, 0.0]).is_err());

        // Invalid movement - Too large
        assert!(validator.validate_movement(&[100.0, 0.0, 0.0]).is_err());
    }

    #[test]
    fn test_sequence_validation() {
        let mut validator = InputValidator::with_default_config();

        // First sequence should be valid
        assert!(validator.validate_sequence("player1", 1).is_ok());

        // Update tracking
        validator.update_tracking("player1", 1, 1000);

        // Next sequence should be valid
        assert!(validator.validate_sequence("player1", 2).is_ok());

        // Duplicate sequence should be invalid
        assert!(validator.validate_sequence("player1", 1).is_err());
    }

    #[test]
    fn test_rate_limiting() {
        let mut validator = InputValidator::new(ValidationConfig {
            max_inputs_per_second: 2,
            ..Default::default()
        });

        // First two inputs should be valid
        assert!(validator.check_rate_limit("player1").is_ok());
        validator.update_tracking("player1", 1, 1000);

        assert!(validator.check_rate_limit("player1").is_ok());
        validator.update_tracking("player1", 2, 1100);

        // Third input should be rate limited
        assert!(validator.check_rate_limit("player1").is_err());
    }
}
