/// Quantization utilities for reducing precision to save bandwidth
/// Uses i16 for positions (range: -32768 to 32767) and i8 for smaller values

use serde::{Deserialize, Serialize};

/// Quantization configuration for different value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationConfig {
    /// Position quantization factor (how many units per i16 step)
    pub position_factor: f32,
    /// Rotation quantization factor (degrees per i8 step)
    pub rotation_factor: f32,
    /// Scale quantization factor (scale per i8 step)
    pub scale_factor: f32,
    /// Velocity quantization factor (units per second per i16 step)
    pub velocity_factor: f32,
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        Self {
            position_factor: 0.01,      // 0.01 units per step = ±327.68 units range
            rotation_factor: 1.0,       // 1 degree per step = ±128 degrees range
            scale_factor: 0.01,         // 0.01 scale per step = ±1.28 scale range
            velocity_factor: 0.1,       // 0.1 units/sec per step = ±3276.8 units/sec range
        }
    }
}

/// Quantize a 3D position vector (f32) to i16
pub fn quantize_position(position: [f32; 3], config: &QuantizationConfig) -> [i16; 3] {
    [
        (position[0] / config.position_factor) as i16,
        (position[1] / config.position_factor) as i16,
        (position[2] / config.position_factor) as i16,
    ]
}

/// Dequantize an i16 position back to f32
pub fn dequantize_position(position: [i16; 3], config: &QuantizationConfig) -> [f32; 3] {
    [
        position[0] as f32 * config.position_factor,
        position[1] as f32 * config.position_factor,
        position[2] as f32 * config.position_factor,
    ]
}

/// Quantize a rotation value (f32 degrees) to i8
pub fn quantize_rotation(rotation: f32, config: &QuantizationConfig) -> i8 {
    (rotation / config.rotation_factor) as i8
}

/// Dequantize an i8 rotation back to f32 degrees
pub fn dequantize_rotation(rotation: i8, config: &QuantizationConfig) -> f32 {
    rotation as f32 * config.rotation_factor
}

/// Quantize a scale value (f32) to i8
pub fn quantize_scale(scale: f32, config: &QuantizationConfig) -> i8 {
    (scale / config.scale_factor) as i8
}

/// Dequantize an i8 scale back to f32
pub fn dequantize_scale(scale: i8, config: &QuantizationConfig) -> f32 {
    scale as f32 * config.scale_factor
}

/// Quantize a 3D velocity vector (f32) to i16
pub fn quantize_velocity(velocity: [f32; 3], config: &QuantizationConfig) -> [i16; 3] {
    [
        (velocity[0] / config.velocity_factor) as i16,
        (velocity[1] / config.velocity_factor) as i16,
        (velocity[2] / config.velocity_factor) as i16,
    ]
}

/// Dequantize an i16 velocity back to f32
pub fn dequantize_velocity(velocity: [i16; 3], config: &QuantizationConfig) -> [f32; 3] {
    [
        velocity[0] as f32 * config.velocity_factor,
        velocity[1] as f32 * config.velocity_factor,
        velocity[2] as f32 * config.velocity_factor,
    ]
}

/// Quantize a small integer value to i8 (for health, ammo, etc.)
pub fn quantize_small_int(value: i32, _config: &QuantizationConfig) -> i8 {
    // Clamp to i8 range and quantize
    let clamped = value.clamp(i8::MIN as i32, i8::MAX as i32);
    clamped as i8
}

/// Dequantize an i8 small integer back to i32
pub fn dequantize_small_int(value: i8, _config: &QuantizationConfig) -> i32 {
    value as i32
}

/// Calculate the size savings from quantization
pub fn calculate_size_savings(original_bytes: usize, quantized_bytes: usize) -> f32 {
    if original_bytes == 0 {
        return 0.0;
    }
    1.0 - (quantized_bytes as f32 / original_bytes as f32)
}

/// Quantized entity transform component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuantizedTransform {
    pub position: [i16; 3],
    pub rotation: i8,
    pub scale: i8,
}

impl QuantizedTransform {
    pub fn new(position: [f32; 3], rotation: f32, scale: f32, config: &QuantizationConfig) -> Self {
        Self {
            position: quantize_position(position, config),
            rotation: quantize_rotation(rotation, config),
            scale: quantize_scale(scale, config),
        }
    }

    pub fn to_original(&self, config: &QuantizationConfig) -> ([f32; 3], f32, f32) {
        (
            dequantize_position(self.position, config),
            dequantize_rotation(self.rotation, config),
            dequantize_scale(self.scale, config),
        )
    }
}

/// Quantized physics component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuantizedPhysics {
    pub velocity: [i16; 3],
    pub angular_velocity: [i16; 3],
    pub mass: i8,
    pub friction: i8,
}

impl QuantizedPhysics {
    pub fn new(
        velocity: [f32; 3],
        angular_velocity: [f32; 3],
        mass: f32,
        friction: f32,
        config: &QuantizationConfig,
    ) -> Self {
        Self {
            velocity: quantize_velocity(velocity, config),
            angular_velocity: quantize_velocity(angular_velocity, config),
            mass: quantize_small_int((mass * 100.0) as i32, config),
            friction: quantize_small_int((friction * 100.0) as i32, config),
        }
    }

    pub fn to_original(&self, config: &QuantizationConfig) -> ([f32; 3], [f32; 3], f32, f32) {
        (
            dequantize_velocity(self.velocity, config),
            dequantize_velocity(self.angular_velocity, config),
            dequantize_small_int(self.mass, config) as f32 / 100.0,
            dequantize_small_int(self.friction, config) as f32 / 100.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_quantization() {
        let config = QuantizationConfig::default();
        let original = [1.5, -2.3, 100.0];
        let quantized = quantize_position(original, &config);
        let dequantized = dequantize_position(quantized, &config);

        // Check that dequantized is close to original (within quantization precision)
        for i in 0..3 {
            let diff = (original[i] - dequantized[i]).abs();
            assert!(diff < config.position_factor * 2.0, "Position {}: original={}, quantized={}, dequantized={}, diff={}", i, original[i], quantized[i], dequantized[i], diff);
        }
    }

    #[test]
    fn test_rotation_quantization() {
        let config = QuantizationConfig::default();
        let original = 45.5;
        let quantized = quantize_rotation(original, &config);
        let dequantized = dequantize_rotation(quantized, &config);

        let diff = (original - dequantized).abs();
        assert!(diff < config.rotation_factor * 2.0);
    }

    #[test]
    fn test_velocity_quantization() {
        let config = QuantizationConfig::default();
        let original = [10.5, -5.2, 0.1];
        let quantized = quantize_velocity(original, &config);
        let dequantized = dequantize_velocity(quantized, &config);

        for i in 0..3 {
            let diff = (original[i] - dequantized[i]).abs();
            assert!(diff < config.velocity_factor * 2.0);
        }
    }

    #[test]
    fn test_size_calculations() {
        // Original: 3 f32 positions + 1 f32 rotation + 1 f32 scale = 5 * 4 = 20 bytes
        let original_bytes = 20;
        // Quantized: 3 i16 positions + 1 i8 rotation + 1 i8 scale = 3 * 2 + 1 + 1 = 8 bytes
        let quantized_bytes = 8;
        let savings = calculate_size_savings(original_bytes, quantized_bytes);
        assert_eq!(savings, 0.6); // 60% size reduction
    }
}
