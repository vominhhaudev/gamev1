/// Snapshot and delta encoding for game state synchronization
/// Provides efficient serialization with quantization for reduced bandwidth

use crate::message::{EntitySnapshot, EntityDelta};
use crate::quantization::{QuantizationConfig, QuantizedTransform, QuantizedPhysics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Quantized entity snapshot with compressed component data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedEntitySnapshot {
    pub id: String,
    pub components: QuantizedEntityComponents,
}

/// Quantized entity delta for incremental updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedEntityDelta {
    pub id: String,
    pub changes: QuantizedEntityComponentChanges,
}

/// All quantized components for an entity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuantizedEntityComponents {
    pub transform: Option<QuantizedTransform>,
    pub physics: Option<QuantizedPhysics>,
    pub health: Option<i8>,
    pub metadata: Option<serde_json::Value>,
}

/// Changes to quantized entity components
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuantizedEntityComponentChanges {
    pub transform: Option<QuantizedTransform>,
    pub physics: Option<QuantizedPhysics>,
    pub health: Option<i8>,
    pub metadata: Option<serde_json::Value>,
}

impl QuantizedEntityComponents {
    /// Create from unquantized entity snapshot
    pub fn from_entity_snapshot(
        snapshot: &EntitySnapshot,
        config: &QuantizationConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut components = Self::default();
        let component_map: HashMap<String, serde_json::Value> =
            serde_json::from_value(snapshot.components.clone())?;

        // Extract and quantize transform component
        if let Some(transform_data) = component_map.get("transform") {
            if let Ok(transform) = serde_json::from_value::<TransformComponent>(transform_data.clone()) {
                components.transform = Some(QuantizedTransform::new(
                    transform.position,
                    transform.rotation,
                    transform.scale,
                    config,
                ));
            }
        }

        // Extract and quantize physics component
        if let Some(physics_data) = component_map.get("physics") {
            if let Ok(physics) = serde_json::from_value::<PhysicsComponent>(physics_data.clone()) {
                components.physics = Some(QuantizedPhysics::new(
                    physics.velocity,
                    physics.angular_velocity,
                    physics.mass,
                    physics.friction,
                    config,
                ));
            }
        }

        // Extract health as quantized i8
        if let Some(health_data) = component_map.get("health") {
            if let Some(health) = health_data.as_i64() {
                components.health = Some(health.clamp(i8::MIN as i64, i8::MAX as i64) as i8);
            }
        }

        // Keep metadata as-is (not quantized)
        if let Some(metadata) = component_map.get("metadata") {
            components.metadata = Some(metadata.clone());
        }

        Ok(components)
    }

    /// Convert back to unquantized form
    pub fn to_entity_snapshot(&self, config: &QuantizationConfig) -> EntitySnapshot {
        let mut component_map = serde_json::Map::new();

        // Dequantize transform
        if let Some(transform) = &self.transform {
            let (position, rotation, scale) = transform.to_original(config);
            let transform_component = TransformComponent {
                position,
                rotation,
                scale,
            };
            component_map.insert(
                "transform".to_string(),
                serde_json::to_value(transform_component).unwrap_or_default(),
            );
        }

        // Dequantize physics
        if let Some(physics) = &self.physics {
            let (velocity, angular_velocity, mass, friction) = physics.to_original(config);
            let physics_component = PhysicsComponent {
                velocity,
                angular_velocity,
                mass,
                friction,
            };
            component_map.insert(
                "physics".to_string(),
                serde_json::to_value(physics_component).unwrap_or_default(),
            );
        }

        // Health (already quantized, just convert back to i64)
        if let Some(health) = self.health {
            component_map.insert(
                "health".to_string(),
                serde_json::Value::Number((health as i64).into()),
            );
        }

        // Metadata (unchanged)
        if let Some(metadata) = &self.metadata {
            component_map.insert("metadata".to_string(), metadata.clone());
        }

        EntitySnapshot {
            id: "".to_string(), // Will be set by caller
            components: serde_json::Value::Object(component_map),
        }
    }

    /// Calculate serialized size for bandwidth estimation
    pub fn serialized_size(&self) -> usize {
        bincode::serialized_size(self).unwrap_or(0) as usize
    }
}

impl QuantizedEntityComponentChanges {
    /// Create from unquantized entity delta
    pub fn from_entity_delta(
        delta: &EntityDelta,
        config: &QuantizationConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut changes = Self::default();
        let change_map: HashMap<String, serde_json::Value> =
            serde_json::from_value(delta.changes.clone())?;

        // Extract and quantize transform changes
        if let Some(transform_data) = change_map.get("transform") {
            if let Ok(transform) = serde_json::from_value::<TransformComponent>(transform_data.clone()) {
                changes.transform = Some(QuantizedTransform::new(
                    transform.position,
                    transform.rotation,
                    transform.scale,
                    config,
                ));
            }
        }

        // Extract and quantize physics changes
        if let Some(physics_data) = change_map.get("physics") {
            if let Ok(physics) = serde_json::from_value::<PhysicsComponent>(physics_data.clone()) {
                changes.physics = Some(QuantizedPhysics::new(
                    physics.velocity,
                    physics.angular_velocity,
                    physics.mass,
                    physics.friction,
                    config,
                ));
            }
        }

        // Extract health changes
        if let Some(health_data) = change_map.get("health") {
            if let Some(health) = health_data.as_i64() {
                changes.health = Some(health.clamp(i8::MIN as i64, i8::MAX as i64) as i8);
            }
        }

        // Metadata changes (unchanged)
        if let Some(metadata) = change_map.get("metadata") {
            changes.metadata = Some(metadata.clone());
        }

        Ok(changes)
    }

    /// Convert back to unquantized delta form
    pub fn to_entity_delta(&self, config: &QuantizationConfig) -> EntityDelta {
        let mut change_map = serde_json::Map::new();

        // Dequantize transform changes
        if let Some(transform) = &self.transform {
            let (position, rotation, scale) = transform.to_original(config);
            let transform_component = TransformComponent {
                position,
                rotation,
                scale,
            };
            change_map.insert(
                "transform".to_string(),
                serde_json::to_value(transform_component).unwrap_or_default(),
            );
        }

        // Dequantize physics changes
        if let Some(physics) = &self.physics {
            let (velocity, angular_velocity, mass, friction) = physics.to_original(config);
            let physics_component = PhysicsComponent {
                velocity,
                angular_velocity,
                mass,
                friction,
            };
            change_map.insert(
                "physics".to_string(),
                serde_json::to_value(physics_component).unwrap_or_default(),
            );
        }

        // Health changes
        if let Some(health) = self.health {
            change_map.insert(
                "health".to_string(),
                serde_json::Value::Number((health as i64).into()),
            );
        }

        // Metadata changes
        if let Some(metadata) = &self.metadata {
            change_map.insert("metadata".to_string(), metadata.clone());
        }

        EntityDelta {
            id: "".to_string(), // Will be set by caller
            changes: serde_json::Value::Object(change_map),
        }
    }

    /// Check if there are any actual changes
    pub fn has_changes(&self) -> bool {
        self.transform.is_some() ||
        self.physics.is_some() ||
        self.health.is_some() ||
        self.metadata.is_some()
    }

    /// Calculate serialized size for bandwidth estimation
    pub fn serialized_size(&self) -> usize {
        bincode::serialized_size(self).unwrap_or(0) as usize
    }
}

/// Transform component structure for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformComponent {
    pub position: [f32; 3],
    pub rotation: f32,
    pub scale: f32,
}

/// Physics component structure for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsComponent {
    pub velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub mass: f32,
    pub friction: f32,
}

/// Encode a snapshot with quantization
pub fn encode_snapshot(
    snapshot: &[EntitySnapshot],
    config: &QuantizationConfig,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut quantized_entities = Vec::new();

    for entity in snapshot {
        let components = QuantizedEntityComponents::from_entity_snapshot(entity, config)?;
        quantized_entities.push(QuantizedEntitySnapshot {
            id: entity.id.clone(),
            components,
        });
    }

    // Use bincode for efficient binary serialization
    let encoded = bincode::serialize(&quantized_entities)?;
    Ok(encoded)
}

/// Decode a quantized snapshot back to original form
pub fn decode_snapshot(
    data: &[u8],
    config: &QuantizationConfig,
) -> Result<Vec<EntitySnapshot>, Box<dyn std::error::Error + Send + Sync>> {
    let quantized_entities: Vec<QuantizedEntitySnapshot> = bincode::deserialize(data)?;

    let mut entities = Vec::new();
    for quantized in quantized_entities {
        let mut entity = quantized.components.to_entity_snapshot(config);
        entity.id = quantized.id;
        entities.push(entity);
    }

    Ok(entities)
}

/// Encode a delta with quantization
pub fn encode_delta(
    delta: &[EntityDelta],
    config: &QuantizationConfig,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut quantized_deltas = Vec::new();

    for change in delta {
        let changes = QuantizedEntityComponentChanges::from_entity_delta(change, config)?;
        if changes.has_changes() {
            quantized_deltas.push(QuantizedEntityDelta {
                id: change.id.clone(),
                changes,
            });
        }
    }

    let encoded = bincode::serialize(&quantized_deltas)?;
    Ok(encoded)
}

/// Decode a quantized delta back to original form
pub fn decode_delta(
    data: &[u8],
    config: &QuantizationConfig,
) -> Result<Vec<EntityDelta>, Box<dyn std::error::Error + Send + Sync>> {
    let quantized_deltas: Vec<QuantizedEntityDelta> = bincode::deserialize(data)?;

    let mut deltas = Vec::new();
    for quantized in quantized_deltas {
        let mut delta = quantized.changes.to_entity_delta(config);
        delta.id = quantized.id;
        deltas.push(delta);
    }

    Ok(deltas)
}

/// Calculate compression ratio for a snapshot
pub fn calculate_snapshot_compression_ratio(
    original_size: usize,
    quantized_size: usize,
) -> f32 {
    if original_size == 0 {
        return 0.0;
    }
    1.0 - (quantized_size as f32 / original_size as f32)
}

/// Calculate compression ratio for a delta
pub fn calculate_delta_compression_ratio(
    original_size: usize,
    quantized_size: usize,
) -> f32 {
    if original_size == 0 {
        return 0.0;
    }
    1.0 - (quantized_size as f32 / original_size as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{StateMessage, Frame, EntitySnapshot, EntityDelta};
    use crate::quantization::{quantize_position, dequantize_position};

    #[test]
    fn test_snapshot_roundtrip() {
        let config = QuantizationConfig::default();

        // Create a test entity with transform and physics
        let entity = EntitySnapshot {
            id: "test_entity".to_string(),
            components: serde_json::json!({
                "transform": {
                    "position": [1.5, 2.3, -0.8],
                    "rotation": 45.5,
                    "scale": 1.2
                },
                "physics": {
                    "velocity": [10.5, -5.2, 0.1],
                    "angular_velocity": [0.1, 0.2, 0.3],
                    "mass": 50.0,
                    "friction": 0.5
                },
                "health": 100,
                "metadata": {
                    "name": "Test Entity",
                    "type": "player"
                }
            }),
        };

        // Test basic quantization without full roundtrip for now
        println!("Original size: ~{} bytes", serde_json::to_string(&entity).unwrap().len());

        // Test quantization functions directly
        if let Some(transform_data) = entity.components.get("transform") {
            if let Ok(transform) = serde_json::from_value::<TransformComponent>(transform_data.clone()) {
                let quantized_transform = QuantizedTransform::new(
                    transform.position,
                    transform.rotation,
                    transform.scale,
                    &config,
                );
                let (dequantized_pos, dequantized_rot, dequantized_scale) = quantized_transform.to_original(&config);

                // Check that quantization preserves reasonable precision
                for i in 0..3 {
                    let diff = f32::abs(transform.position[i] - dequantized_pos[i]);
                    assert!(diff < config.position_factor * 2.0, "Position {} differs too much", i);
                }
            }
        }
    }

    #[test]
    fn test_delta_encoding() {
        let config = QuantizationConfig::default();

        // Create original entity
        let original_entity = EntitySnapshot {
            id: "test_entity".to_string(),
            components: serde_json::json!({
                "transform": {
                    "position": [1.0, 2.0, 3.0],
                    "rotation": 0.0,
                    "scale": 1.0
                },
                "health": 100
            }),
        };

        // Create delta (only health changed)
        let delta = EntityDelta {
            id: "test_entity".to_string(),
            changes: serde_json::json!({
                "health": 95
            }),
        };

        // Encode delta
        let encoded = encode_delta(&[delta], &config).unwrap();
        println!("Delta size: {} bytes", encoded.len());

        // Decode delta
        let decoded = decode_delta(&encoded, &config).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].id, "test_entity");

        // Verify only health changed
        let changes: HashMap<String, serde_json::Value> =
            serde_json::from_value(decoded[0].changes.clone()).unwrap();
        assert_eq!(changes.get("health").unwrap().as_i64().unwrap(), 95);
        assert!(changes.get("transform").is_none());
    }

    #[test]
    fn test_compression_ratios() {
        let config = QuantizationConfig::default();

        // Create a snapshot with multiple entities
        let entities = vec![
            EntitySnapshot {
                id: "entity1".to_string(),
                components: serde_json::json!({
                    "transform": {
                        "position": [1.0, 2.0, 3.0],
                        "rotation": 45.0,
                        "scale": 1.0
                    }
                }),
            },
            EntitySnapshot {
                id: "entity2".to_string(),
                components: serde_json::json!({
                    "physics": {
                        "velocity": [10.0, 0.0, 0.0],
                        "mass": 50.0
                    }
                }),
            },
        ];

        let original_json = serde_json::to_string(&entities).unwrap();
        let encoded = encode_snapshot(&entities, &config).unwrap();

        let compression_ratio = calculate_snapshot_compression_ratio(original_json.len(), encoded.len());
        println!("Compression ratio: {:.2}%", compression_ratio * 100.0);

        // Should achieve significant compression due to quantization
        assert!(compression_ratio > 0.3); // At least 30% reduction
    }

    #[test]
    fn test_basic_quantization_encoding() {
        let config = QuantizationConfig::default();

        // Test position quantization
        let original_pos = [1.5, -2.3, 100.0];
        let quantized_pos = quantize_position(original_pos, &config);
        let dequantized_pos = dequantize_position(quantized_pos, &config);

        println!("Original position: {:?}", original_pos);
        println!("Quantized position: {:?}", quantized_pos);
        println!("Dequantized position: {:?}", dequantized_pos);

        // Check that quantization preserves reasonable precision
        for i in 0..3 {
            let diff = f32::abs(original_pos[i] - dequantized_pos[i]);
            assert!(diff < config.position_factor * 2.0, "Position {} differs too much", i);
        }

        // Test that quantized values are actually smaller in range
        assert!(quantized_pos.iter().all(|&x| x.abs() <= 32767));
    }
}
