
use bevy_ecs::prelude::*;
use rapier3d::prelude::*;
use rapier3d::geometry::DefaultBroadPhase;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Components cho gameplay
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct TransformQ {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // quaternion
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct VelocityQ {
    pub velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub score: u32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Pickup {
    pub value: u32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Lifetime {
    pub remaining: Duration,
}

/// Input từ client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInput {
    pub player_id: String,
    pub input_sequence: u32,
    pub movement: [f32; 3], // x, y, z movement
    pub timestamp: u64,
}

/// Snapshot gửi về client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub tick: u64,
    pub entities: Vec<EntitySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: u32,
    pub transform: TransformQ,
    pub velocity: Option<VelocityQ>,
    pub player: Option<Player>,
    pub pickup: Option<Pickup>,
}

/// Simplified simulation world for basic testing
pub struct SimulationWorld {
    pub tick_count: u64,
    pub entities: Vec<EntitySnapshot>,
}

impl SimulationWorld {
    pub fn new() -> Self {
        Self {
            tick_count: 0,
            entities: Vec::new(),
        }
    }

    pub fn step(&mut self, _delta_time: Duration) {
        self.tick_count += 1;

        // Add a simple entity for testing if none exist (no logging in main loop)
        if self.entities.is_empty() {
            self.entities.push(EntitySnapshot {
                id: 0,
                transform: TransformQ {
                    position: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                },
                velocity: None,
                player: None,
                pickup: None,
            });
        }
    }

    pub fn create_snapshot(&self) -> GameSnapshot {
        GameSnapshot {
            tick: self.tick_count,
            entities: self.entities.clone(),
        }
    }
}

// TODO: Add systems and input buffer when bevy_ecs integration is stable
