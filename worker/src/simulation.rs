
use bevy_ecs::prelude::*;
use rapier3d::prelude::*;
use rapier3d::geometry::DefaultBroadPhase;
use rapier3d::dynamics::{MultibodyJointSet, ImpulseJointSet};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::{Duration, Instant}};
use tracing;

use crate::validation::InputValidator;

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
    pub view_distance: f32, // Area of Interest radius
    pub last_position: [f32; 3], // For movement tracking
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Pickup {
    pub value: u32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Lifetime {
    pub remaining: Duration,
}

#[derive(Component, Debug, Clone)]
pub struct RigidBodyHandle {
    pub handle: rapier3d::dynamics::RigidBodyHandle,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Obstacle {
    pub obstacle_type: String, // "wall", "spike", "moving_platform"
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct PowerUp {
    pub power_type: String, // "speed_boost", "jump_boost", "invincibility"
    pub duration: Duration,
    pub value: u32,
}

#[derive(Component, Debug, Clone)]
pub struct Enemy {
    pub enemy_type: String, // "basic", "fast", "tank"
    pub damage: u32,
    pub speed: f32,
    pub last_attack: Instant,
    pub attack_cooldown: Duration,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub player_id: String,
    pub player_name: String,
    pub message: String,
    pub timestamp: u64,
    pub message_type: ChatMessageType,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Spectator {
    pub id: String,
    pub target_player_id: Option<String>, // Player đang follow (nếu có)
    pub camera_mode: SpectatorCameraMode,
    pub view_distance: f32,
    pub last_position: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpectatorCameraMode {
    Free,        // Camera tự do di chuyển
    Follow,      // Follow theo player cụ thể
    Overview,    // Camera overview toàn bộ game
    Fixed,       // Camera cố định tại vị trí
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatMessageType {
    Global,    // Message gửi tới tất cả players
    Team,      // Message gửi tới team members
    Whisper,   // Private message tới player cụ thể
    System,    // System announcement
}

// ===== QUANTIZATION & DELTA ENCODING SYSTEM =====

// Quantization parameters
pub const POSITION_SCALE: f32 = 100.0; // Scale factor để chuyển f32 thành i16
pub const ROTATION_SCALE: f32 = 10000.0; // Scale factor cho quaternion components
pub const VELOCITY_SCALE: f32 = 50.0; // Scale factor cho velocity

/// Quantized transform để giảm kích thước dữ liệu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedTransform {
    pub position: (i16, i16, i16),  // x, y, z quantized positions
    pub rotation: (i16, i16, i16, i16), // quaternion components quantized
}

/// Quantized velocity để giảm kích thước dữ liệu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedVelocity {
    pub velocity: (i16, i16, i16),  // x, y, z velocity components
    pub angular_velocity: (i16, i16, i16), // angular velocity components
}

/// Quantized entity snapshot để giảm băng thông
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedEntitySnapshot {
    pub id: u32,
    pub transform: QuantizedTransform,
    pub velocity: Option<QuantizedVelocity>,
    pub player: Option<QuantizedPlayer>,
    pub pickup: Option<QuantizedPickup>,
    pub obstacle: Option<QuantizedObstacle>,
    pub power_up: Option<QuantizedPowerUp>,
    pub enemy: Option<QuantizedEnemy>,
}

/// Quantized player data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedPlayer {
    pub id: String,
    pub score: u32,
    pub view_distance: i16, // quantized view distance
}

/// Quantized pickup data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedPickup {
    pub value: u32,
}

/// Quantized obstacle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedObstacle {
    pub obstacle_type: String,
}

/// Quantized power-up data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedPowerUp {
    pub power_type: String,
    pub duration: u16, // quantized duration in ticks
    pub value: u32,
}

/// Quantized enemy data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedEnemy {
    pub enemy_type: String,
    pub damage: u32,
    pub speed: i16, // quantized speed
}

/// Delta snapshot - chỉ chứa dữ liệu đã thay đổi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaSnapshot {
    pub tick: u64,
    pub base_tick: u64, // Reference tick cho delta
    pub created_entities: Vec<QuantizedEntitySnapshot>, // Entities mới được tạo
    pub updated_entities: Vec<QuantizedEntitySnapshot>, // Entities có thay đổi
    pub deleted_entities: Vec<u32>, // Entity IDs bị xóa
    pub chat_messages: Vec<ChatMessage>, // Chat messages mới
    pub new_spectators: Vec<SpectatorSnapshot>, // Spectators mới
    pub removed_spectators: Vec<String>, // Spectator IDs bị xóa
}

/// Full snapshot với quantization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedSnapshot {
    pub tick: u64,
    pub entities: Vec<QuantizedEntitySnapshot>,
    pub chat_messages: Vec<ChatMessage>,
    pub spectators: Vec<SpectatorSnapshot>,
}

/// Quantization utilities
impl QuantizedTransform {
    /// Convert f32 position to i16 với scale factor
    pub fn from_f32(position: [f32; 3], rotation: [f32; 4]) -> Self {
        Self {
            position: (
                (position[0] * POSITION_SCALE) as i16,
                (position[1] * POSITION_SCALE) as i16,
                (position[2] * POSITION_SCALE) as i16,
            ),
            rotation: (
                (rotation[0] * ROTATION_SCALE) as i16,
                (rotation[1] * ROTATION_SCALE) as i16,
                (rotation[2] * ROTATION_SCALE) as i16,
                (rotation[3] * ROTATION_SCALE) as i16,
            ),
        }
    }

    /// Convert i16 back to f32
    pub fn to_f32(&self) -> ([f32; 3], [f32; 4]) {
        (
            [
                self.position.0 as f32 / POSITION_SCALE,
                self.position.1 as f32 / POSITION_SCALE,
                self.position.2 as f32 / POSITION_SCALE,
            ],
            [
                self.rotation.0 as f32 / ROTATION_SCALE,
                self.rotation.1 as f32 / ROTATION_SCALE,
                self.rotation.2 as f32 / ROTATION_SCALE,
                self.rotation.3 as f32 / ROTATION_SCALE,
            ],
        )
    }
}

impl QuantizedVelocity {
    /// Convert f32 velocity to i16
    pub fn from_f32(velocity: [f32; 3], angular_velocity: [f32; 3]) -> Self {
        Self {
            velocity: (
                (velocity[0] * VELOCITY_SCALE) as i16,
                (velocity[1] * VELOCITY_SCALE) as i16,
                (velocity[2] * VELOCITY_SCALE) as i16,
            ),
            angular_velocity: (
                (angular_velocity[0] * VELOCITY_SCALE) as i16,
                (angular_velocity[1] * VELOCITY_SCALE) as i16,
                (angular_velocity[2] * VELOCITY_SCALE) as i16,
            ),
        }
    }

    /// Convert i16 back to f32
    pub fn to_f32(&self) -> ([f32; 3], [f32; 3]) {
        (
            [
                self.velocity.0 as f32 / VELOCITY_SCALE,
                self.velocity.1 as f32 / VELOCITY_SCALE,
                self.velocity.2 as f32 / VELOCITY_SCALE,
            ],
            [
                self.angular_velocity.0 as f32 / VELOCITY_SCALE,
                self.angular_velocity.1 as f32 / VELOCITY_SCALE,
                self.angular_velocity.2 as f32 / VELOCITY_SCALE,
            ],
        )
    }
}

/// Delta encoder để tính toán sự khác biệt giữa snapshots
pub struct DeltaEncoder {
    /// Previous snapshot để so sánh
    pub previous_snapshot: Option<QuantizedSnapshot>,
    /// Threshold để quyết định có nên tạo delta hay không
    pub delta_threshold: usize, // Số entities thay đổi tối thiểu để tạo delta
}

impl DeltaEncoder {
    pub fn new(delta_threshold: usize) -> Self {
        Self {
            previous_snapshot: None,
            delta_threshold,
        }
    }

    /// Encode snapshot thành delta hoặc full snapshot
    pub fn encode_snapshot(&mut self, snapshot: GameSnapshot, current_tick: u64) -> EncodedSnapshot {
        let quantized = self.quantize_snapshot(snapshot);

        if let Some(ref prev) = self.previous_snapshot {
            // Tính toán delta nếu có đủ sự thay đổi
            let delta = self.create_delta(&quantized, prev, current_tick);
            if self.should_use_delta(&delta) {
                EncodedSnapshot::Delta(delta)
            } else {
                // Gửi full snapshot nếu delta quá lớn
                self.previous_snapshot = Some(quantized.clone());
                EncodedSnapshot::Full(quantized)
            }
        } else {
            // First snapshot luôn là full
            self.previous_snapshot = Some(quantized.clone());
            EncodedSnapshot::Full(quantized)
        }
    }

    /// Quantize GameSnapshot thành QuantizedSnapshot
    fn quantize_snapshot(&self, snapshot: GameSnapshot) -> QuantizedSnapshot {
        let entities = snapshot.entities.into_iter().map(|entity| {
            let quantized_transform = QuantizedTransform::from_f32(
                entity.transform.position,
                entity.transform.rotation,
            );

            let quantized_velocity = entity.velocity.map(|vel| {
                QuantizedVelocity::from_f32(vel.velocity, vel.angular_velocity)
            });

            QuantizedEntitySnapshot {
                id: entity.id,
                transform: quantized_transform,
                velocity: quantized_velocity,
                player: entity.player.map(|p| QuantizedPlayer {
                    id: p.id,
                    score: p.score,
                    view_distance: (p.view_distance * POSITION_SCALE) as i16,
                }),
                pickup: entity.pickup.map(|p| QuantizedPickup { value: p.value }),
                obstacle: entity.obstacle.map(|o| QuantizedObstacle { obstacle_type: o.obstacle_type }),
                power_up: entity.power_up.map(|pu| QuantizedPowerUp {
                    power_type: pu.power_type,
                    duration: (pu.duration.as_secs() * 60) as u16, // Convert to ticks
                    value: pu.value,
                }),
                enemy: entity.enemy.map(|e| QuantizedEnemy {
                    enemy_type: e.enemy_type,
                    damage: e.damage,
                    speed: (e.speed * VELOCITY_SCALE) as i16,
                }),
            }
        }).collect();

        QuantizedSnapshot {
            tick: snapshot.tick,
            entities,
            chat_messages: snapshot.chat_messages,
            spectators: snapshot.spectators,
        }
    }

    /// Tạo delta từ current snapshot so với previous
    fn create_delta(&self, current: &QuantizedSnapshot, previous: &QuantizedSnapshot, current_tick: u64) -> DeltaSnapshot {
        // Find created entities (in current but not in previous)
        let mut created_entities = Vec::new();
        for entity in &current.entities {
            if !previous.entities.iter().any(|e| e.id == entity.id) {
                created_entities.push(entity.clone());
            }
        }

        // Find updated entities (exist in both but different)
        let mut updated_entities = Vec::new();
        for current_entity in &current.entities {
            if let Some(prev_entity) = previous.entities.iter().find(|e| e.id == current_entity.id) {
                // Compare nếu có sự khác biệt đáng kể
                if self.has_significant_change(current_entity, prev_entity) {
                    updated_entities.push(current_entity.clone());
                }
            }
        }

        // Find deleted entities (in previous but not in current)
        let mut deleted_entities = Vec::new();
        for prev_entity in &previous.entities {
            if !current.entities.iter().any(|e| e.id == prev_entity.id) {
                deleted_entities.push(prev_entity.id);
            }
        }

        // New chat messages
        let mut new_chat_messages = Vec::new();
        for chat_msg in &current.chat_messages {
            if !previous.chat_messages.iter().any(|m| m.id == chat_msg.id) {
                new_chat_messages.push(chat_msg.clone());
            }
        }

        // New spectators
        let mut new_spectators = Vec::new();
        for spectator in &current.spectators {
            if !previous.spectators.iter().any(|s| s.id == spectator.id) {
                new_spectators.push(spectator.clone());
            }
        }

        // Removed spectators
        let mut removed_spectators = Vec::new();
        for prev_spectator in &previous.spectators {
            if !current.spectators.iter().any(|s| s.id == prev_spectator.id) {
                removed_spectators.push(prev_spectator.id.clone());
            }
        }

        DeltaSnapshot {
            tick: current.tick,
            base_tick: previous.tick,
            created_entities,
            updated_entities,
            deleted_entities,
            chat_messages: new_chat_messages,
            new_spectators,
            removed_spectators,
        }
    }

    /// Check if entity có sự thay đổi đáng kể để gửi delta
    fn has_significant_change(&self, current: &QuantizedEntitySnapshot, previous: &QuantizedEntitySnapshot) -> bool {
        // Check position change
        let pos_diff_x = (current.transform.position.0 - previous.transform.position.0).abs() > 1;
        let pos_diff_y = (current.transform.position.1 - previous.transform.position.1).abs() > 1;
        let pos_diff_z = (current.transform.position.2 - previous.transform.position.2).abs() > 1;

        // Check velocity change (nếu có)
        let vel_changed = match (&current.velocity, &previous.velocity) {
            (Some(curr_vel), Some(prev_vel)) => {
                (curr_vel.velocity.0 - prev_vel.velocity.0).abs() > 2 ||
                (curr_vel.velocity.1 - prev_vel.velocity.1).abs() > 2 ||
                (curr_vel.velocity.2 - prev_vel.velocity.2).abs() > 2
            }
            (Some(_), None) | (None, Some(_)) => true,
            (None, None) => false,
        };

        pos_diff_x || pos_diff_y || pos_diff_z || vel_changed
    }

    /// Decide có nên sử dụng delta hay không dựa trên kích thước
    fn should_use_delta(&self, delta: &DeltaSnapshot) -> bool {
        let total_changes = delta.created_entities.len() + delta.updated_entities.len() + delta.deleted_entities.len();
        total_changes >= self.delta_threshold
    }
}

/// Encoded snapshot - có thể là full hoặc delta
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EncodedSnapshot {
    Full(QuantizedSnapshot),
    Delta(DeltaSnapshot),
}

impl EncodedSnapshot {
    /// Get tick number from snapshot
    pub fn tick(&self) -> u64 {
        match self {
            EncodedSnapshot::Full(snapshot) => snapshot.tick,
            EncodedSnapshot::Delta(delta) => delta.tick,
        }
    }

    /// Get payload as JSON string
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

// ===== AOI (Area of Interest) System =====

/// Grid cell coordinates (x, z) - chỉ dùng 2D cho simplicity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCell {
    pub x: i32,
    pub z: i32,
}

/// Grid-based spatial partitioning system
#[derive(Debug)]
pub struct SpatialGrid {
    /// Cell size in world units (ví dụ: 50.0)
    pub cell_size: f32,
    /// Map từ cell coordinates tới list of entities
    pub cells: HashMap<GridCell, Vec<Entity>>,
    /// Cache để track entity positions để detect movement
    pub entity_positions: HashMap<Entity, [f32; 3]>,
}

/// Player's Area of Interest - các cells mà player có thể thấy
#[derive(Debug, Clone)]
pub struct PlayerAOI {
    pub player_entity: Entity,
    pub visible_cells: Vec<GridCell>,
    pub last_update_tick: u64,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
            entity_positions: HashMap::new(),
        }
    }

    /// Convert world position to grid cell coordinates
    pub fn world_to_cell(&self, position: [f32; 3]) -> GridCell {
        GridCell {
            x: (position[0] / self.cell_size).floor() as i32,
            z: (position[2] / self.cell_size).floor() as i32,
        }
    }

    /// Add entity to grid at specific position
    pub fn add_entity(&mut self, entity: Entity, position: [f32; 3]) {
        let cell = self.world_to_cell(position);

        // Remove from old position if exists
        if let Some(old_pos) = self.entity_positions.get(&entity) {
            let old_cell = self.world_to_cell(*old_pos);
            if let Some(entities) = self.cells.get_mut(&old_cell) {
                entities.retain(|&e| e != entity);
                if entities.is_empty() {
                    self.cells.remove(&old_cell);
                }
            }
        }

        // Add to new position
        self.cells.entry(cell).or_insert_with(Vec::new).push(entity);
        self.entity_positions.insert(entity, position);
    }

    /// Remove entity from grid
    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some(position) = self.entity_positions.remove(&entity) {
            let cell = self.world_to_cell(position);
            if let Some(entities) = self.cells.get_mut(&cell) {
                entities.retain(|&e| e != entity);
                if entities.is_empty() {
                    self.cells.remove(&cell);
                }
            }
        }
    }

    /// Update entity position in grid
    pub fn update_entity_position(&mut self, entity: Entity, new_position: [f32; 3]) {
        let old_cell = self.entity_positions.get(&entity).map(|pos| self.world_to_cell(*pos));
        let new_cell = self.world_to_cell(new_position);

        // Nếu cell không đổi, chỉ cần cập nhật position
        if old_cell == Some(new_cell) {
            self.entity_positions.insert(entity, new_position);
            return;
        }

        // Nếu cell thay đổi, cần di chuyển entity
        if let Some(old_cell) = old_cell {
            if let Some(entities) = self.cells.get_mut(&old_cell) {
                entities.retain(|&e| e != entity);
                if entities.is_empty() {
                    self.cells.remove(&old_cell);
                }
            }
        }

        // Add to new cell
        self.cells.entry(new_cell).or_insert_with(Vec::new).push(entity);
        self.entity_positions.insert(entity, new_position);
    }

    /// Get all entities in a specific cell
    pub fn get_entities_in_cell(&self, cell: GridCell) -> Option<&Vec<Entity>> {
        self.cells.get(&cell)
    }

    /// Get all entities in a cell and its 8 neighbors (3x3 grid)
    pub fn get_entities_in_aoi(&self, center_cell: GridCell) -> Vec<Entity> {
        let mut entities = Vec::new();

        // Check center cell and 8 neighbors
        for dx in -1..=1 {
            for dz in -1..=1 {
                let cell = GridCell {
                    x: center_cell.x + dx,
                    z: center_cell.z + dz,
                };

                if let Some(cell_entities) = self.cells.get(&cell) {
                    entities.extend(cell_entities.iter().copied());
                }
            }
        }

        entities
    }

    /// Get player's AOI cells (center cell + neighbors)
    pub fn get_player_aoi_cells(&self, player_position: [f32; 3]) -> Vec<GridCell> {
        let center_cell = self.world_to_cell(player_position);
        let mut cells = Vec::new();

        // Get 3x3 grid of cells around player
        for dx in -1..=1 {
            for dz in -1..=1 {
                cells.push(GridCell {
                    x: center_cell.x + dx,
                    z: center_cell.z + dz,
                });
            }
        }

        cells
    }

    /// Cleanup empty cells to save memory
    pub fn cleanup_empty_cells(&mut self) {
        self.cells.retain(|_, entities| !entities.is_empty());
    }
}

// Simplified version for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemySnapshot {
    pub enemy_type: String,
    pub damage: u32,
    pub speed: f32,
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
    pub chat_messages: Vec<ChatMessage>,
    pub spectators: Vec<SpectatorSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: u32,
    pub transform: TransformQ,
    pub velocity: Option<VelocityQ>,
    pub player: Option<Player>,
    pub pickup: Option<Pickup>,
    pub obstacle: Option<Obstacle>,
    pub power_up: Option<PowerUp>,
    pub enemy: Option<EnemySnapshot>, // Simplified version for serialization
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectatorSnapshot {
    pub id: String,
    pub transform: TransformQ,
    pub camera_mode: String,
    pub target_player_id: Option<String>,
    pub view_distance: f32,
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
                obstacle: None,
                power_up: None,
                enemy: None,
            });
        }
    }

    pub fn create_snapshot(&self) -> GameSnapshot {
        GameSnapshot {
            tick: self.tick_count,
            entities: self.entities.clone(),
            chat_messages: Vec::new(), // SimulationWorld doesn't have chat
            spectators: Vec::new(), // SimulationWorld doesn't have spectators
        }
    }
}

/// Input buffer để xử lý network latency
#[derive(Debug, Clone)]
pub struct InputBuffer {
    pub inputs: Vec<PlayerInput>,
    pub last_processed_sequence: u32,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            last_processed_sequence: 0,
        }
    }

    pub fn add_input(&mut self, input: PlayerInput) {
        // Insert theo sequence number
        let insert_pos = self.inputs.partition_point(|i| i.input_sequence < input.input_sequence);
        self.inputs.insert(insert_pos, input);
    }

    pub fn get_pending_inputs(&self) -> Vec<&PlayerInput> {
        self.inputs.iter()
            .filter(|input| input.input_sequence > self.last_processed_sequence)
            .collect()
    }

    pub fn mark_processed(&mut self, sequence: u32) {
        self.last_processed_sequence = sequence;
        // Remove các input đã xử lý
        self.inputs.retain(|input| input.input_sequence > self.last_processed_sequence);
    }
}

/// Game world với ECS và Physics
pub struct GameWorld {
    pub world: World,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub chat_messages: Vec<ChatMessage>,
    pub query_pipeline: QueryPipeline,
    pub input_buffers: std::collections::HashMap<String, InputBuffer>,
    pub input_validator: InputValidator,
    pub last_tick: Instant,
    pub accumulator: Duration,
    pub tick_rate: Duration, // 60Hz = 16.67ms per tick
    pub spatial_grid: SpatialGrid, // AOI system
    pub player_aois: HashMap<String, PlayerAOI>, // Track each player's AOI
    pub delta_encoder: DeltaEncoder, // Delta encoding system
    pub last_keyframe_tick: u64, // Last time we sent a full snapshot
    pub current_tick: u64, // Current tick count (separate from world resource)
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWorld {
    pub fn new() -> Self {
        let mut world = World::new();

        // Register components và resources
        world.insert_resource(InputBuffers::default());
        world.insert_resource(PlayerEntityMap::default());
        world.insert_resource(TickCount(0));

        // Initialize physics
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let bodies = RigidBodySet::new();
        let colliders = ColliderSet::new();
        let impulse_joints = ImpulseJointSet::new();
        let multibody_joints = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();
        let query_pipeline = QueryPipeline::new();

        Self {
            world,
            physics_pipeline,
            island_manager,
            broad_phase,
            narrow_phase,
            bodies,
            colliders,
            impulse_joints,
            multibody_joints,
            ccd_solver,
            chat_messages: Vec::new(),
            query_pipeline,
            input_buffers: std::collections::HashMap::new(),
            input_validator: InputValidator::with_default_config(),
            last_tick: Instant::now(),
            accumulator: Duration::from_secs(0),
            tick_rate: Duration::from_millis(16), // 60Hz
            spatial_grid: SpatialGrid::new(50.0), // 50 unit cells
            player_aois: HashMap::new(),
            delta_encoder: DeltaEncoder::new(5), // Delta threshold: 5 entities
            last_keyframe_tick: 0,
            current_tick: 0,
        }
    }

    /// Main game loop với fixed timestep và delta encoding
    pub fn tick(&mut self) -> EncodedSnapshot {
        let now = std::time::Instant::now();
        self.accumulator += now - self.last_tick;
        self.last_tick = now;

        // Fixed timestep - chỉ tick khi đủ thời gian
        let mut ticks = 0;
        while self.accumulator >= self.tick_rate && ticks < 3 { // Max 3 ticks per frame
            self.fixed_update();
            self.current_tick += 1; // Increment tick count
            self.accumulator -= self.tick_rate;
            ticks += 1;
        }

        // Get current tick count
        let current_tick = self.current_tick;

        // Create base snapshot for encoding
        let base_snapshot = self.create_snapshot();

        // Use delta encoding
        self.delta_encoder.encode_snapshot(base_snapshot, current_tick)
    }

    /// Chạy simulation trong thời gian ngắn để test
    pub fn run_simulation_for_test(&mut self, duration_secs: f32) -> Vec<EncodedSnapshot> {
        let mut snapshots = Vec::new();
        let target_ticks = (duration_secs * 60.0) as u32; // 60 FPS

        // Đảm bảo có ít nhất một số entities để test
        if self.world.query::<&Player>().iter(&self.world).count() == 0 {
            self.add_player("test_player".to_string());
        }

        // Lưu tick count hiện tại để kiểm tra sau
        let initial_tick = self.get_current_tick();

        // Force tick bằng cách set accumulator đủ lớn
        self.accumulator = self.tick_rate * target_ticks as u32;

        for _i in 0..target_ticks {
            // Get encoded snapshot (includes delta encoding)
            let snapshot = self.tick();
            snapshots.push(snapshot);
        }

        // Debug: kiểm tra tick count đã tăng chưa
        let final_tick = self.get_current_tick();
        println!("Initial tick: {}, Final tick: {}, Target ticks: {}", initial_tick, final_tick, target_ticks);

        snapshots
    }

    /// Get current snapshot without mutating state (for external use) - deprecated, use get_snapshot_for_player
    pub fn get_snapshot(&mut self) -> GameSnapshot {
        // Fallback to full snapshot for backward compatibility
        self.create_snapshot()
    }

    /// Get current tick count
    pub fn get_current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Force send keyframe (full snapshot) for specific player
    pub fn force_keyframe_for_player(&mut self, player_id: &str) -> EncodedSnapshot {
        // Create fresh delta encoder for this player
        let mut player_encoder = DeltaEncoder::new(1); // Always send full for keyframe

        let base_snapshot = self.create_snapshot();
        let current_tick = self.world.resource::<TickCount>().0;

        player_encoder.encode_snapshot(base_snapshot, current_tick)
    }

    /// Get current snapshot for a specific player using AOI optimization và delta encoding
    pub fn get_snapshot_for_player(&mut self, player_id: &str) -> EncodedSnapshot {
        let player_position = self.get_player_position(player_id)
            .unwrap_or([0.0, 5.0, 0.0]);

        // Update player's AOI tracking
        self.update_player_aoi_grid(player_id);

        // Get entities in player's AOI using spatial grid
        let aoi_entities = if let Some(player_aoi) = self.player_aois.get(player_id) {
            let center_cell = self.spatial_grid.world_to_cell(player_position);
            self.spatial_grid.get_entities_in_aoi(center_cell)
        } else {
            // Fallback: get all entities if player not tracked
            let mut all_entities = Vec::new();
            let mut query = self.world.query::<Entity>();
            for entity in query.iter(&self.world) {
                all_entities.push(entity);
            }
            all_entities
        };

        // Create AOI-optimized snapshot
        let mut entities = Vec::new();
        for &entity in &aoi_entities {
            // Get entity components
            if let Ok((transform, player, pickup, obstacle, power_up, enemy)) = self.world.query::<(
                &TransformQ,
                Option<&Player>,
                Option<&Pickup>,
                Option<&Obstacle>,
                Option<&PowerUp>,
                Option<&Enemy>
            )>().get(&self.world, entity) {
                entities.push(EntitySnapshot {
                    id: entity.index(),
                    transform: transform.clone(),
                    velocity: self.world.get::<VelocityQ>(entity).cloned(),
                    player: player.cloned(),
                    pickup: pickup.cloned(),
                    obstacle: obstacle.cloned(),
                    power_up: power_up.cloned(),
                    enemy: enemy.map(|e| EnemySnapshot {
                        enemy_type: e.enemy_type.clone(),
                        damage: e.damage,
                        speed: e.speed,
                    }),
                });
            }
        }

        let base_snapshot = GameSnapshot {
            tick: self.world.resource::<TickCount>().0,
            entities,
            chat_messages: self.get_recent_chat_messages(20),
            spectators: self.get_spectator_snapshots(),
        };

        // Use delta encoding for this player's snapshot
        let current_tick = self.world.resource::<TickCount>().0;
        self.delta_encoder.encode_snapshot(base_snapshot, current_tick)
    }

    /// Update player's AOI tracking (called during snapshot generation) - DEPRECATED
    /// Use update_player_aoi_grid instead

    /// Add a chat message to the game world
    pub fn add_chat_message(&mut self, message: ChatMessage) {
        self.chat_messages.push(message);

        // Keep only last 100 messages to prevent memory bloat
        if self.chat_messages.len() > 100 {
            self.chat_messages.drain(0..self.chat_messages.len() - 100);
        }
    }

    /// Get recent chat messages (last N messages)
    pub fn get_recent_chat_messages(&self, count: usize) -> Vec<ChatMessage> {
        let start = if self.chat_messages.len() > count {
            self.chat_messages.len() - count
        } else {
            0
        };
        self.chat_messages[start..].to_vec()
    }

    /// Get spectator snapshots for all active spectators
    pub fn get_spectator_snapshots(&mut self) -> Vec<SpectatorSnapshot> {
        let mut query = self.world.query::<(Entity, &Spectator, &TransformQ)>();
        let mut snapshots = Vec::new();

        for (entity, spectator, transform) in query.iter(&self.world) {
            snapshots.push(SpectatorSnapshot {
                id: spectator.id.clone(),
                transform: TransformQ {
                    position: transform.position,
                    rotation: transform.rotation,
                },
                camera_mode: format!("{:?}", spectator.camera_mode),
                target_player_id: spectator.target_player_id.clone(),
                view_distance: spectator.view_distance,
            });
        }

        snapshots
    }

    fn fixed_update(&mut self) {
        // Tăng tick count (already done in tick() method)
        // current_tick is incremented in tick() method

        // 1. Ingest và validate inputs
        self.ingest_inputs();

        // 2. Validate inputs (anti-cheat cơ bản)
        self.validate_inputs();

        // 3. Endless Runner specific logic (auto-run, procedural generation)
        let delta_time = self.tick_rate;
        self.update_endless_runner(delta_time);

        // 4. Physics step
        self.physics_step();

        // 4.5. Update spatial grid với vị trí mới sau physics
        self.update_spatial_grid();

        // 5. Gameplay logic (collision detection, etc.)
        self.gameplay_logic();

        // 6. Cleanup (lifetime, etc.)
        self.cleanup();

        // 7. Spatial grid maintenance (every 60 ticks)
        if self.current_tick % 60 == 0 {
            self.spatial_grid.cleanup_empty_cells();
        }

        // 8. Room cleanup
        // Note: RoomManager cleanup is handled separately in RPC service
    }

    fn ingest_inputs(&mut self) {
        // Clean up validator periodically
        self.input_validator.cleanup();

        // Collect input applications first to avoid borrowing conflicts
        let mut input_applications = Vec::new();

        for (player_id, buffer) in &mut self.input_buffers {
            let pending_inputs = buffer.get_pending_inputs();

            // Validate and process inputs for this player
            for input in pending_inputs {
                match self.input_validator.validate_input(input) {
                    Ok(_) => {
                        // Input is valid, use it
                        if let Some(player_entity) = self.world.resource::<PlayerEntityMap>().map.get(player_id) {
                            input_applications.push((*player_entity, input.movement[0] * 10.0, input.movement[2] * 10.0));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Invalid input from player {}: {}", player_id, e);
                        // Continue processing other inputs, don't break the game
                    }
                }
            }
        }

        // Apply inputs after collecting and validating
        for (player_entity, vel_x, vel_z) in input_applications {
            if let Some(mut velocity) = self.world.get_mut::<VelocityQ>(player_entity) {
                velocity.velocity[0] = vel_x;
                velocity.velocity[2] = vel_z;
            }
        }
    }

    fn validate_inputs(&mut self) {
        // Anti-cheat cơ bản: clamp velocity
        for mut velocity in self.world.query::<&mut VelocityQ>().iter_mut(&mut self.world) {
            // Clamp velocity để tránh cheating
            let max_speed = 15.0;
            let speed = (velocity.velocity[0].powi(2) + velocity.velocity[2].powi(2)).sqrt();
            if speed > max_speed {
                velocity.velocity[0] *= max_speed / speed;
                velocity.velocity[2] *= max_speed / speed;
            }
        }
    }

    fn physics_step(&mut self) {
        // Rapier physics step
        self.physics_pipeline.step(
            &vector![0.0, -9.81, 0.0], // gravity
            &IntegrationParameters {
                dt: self.tick_rate.as_secs_f32(),
                ..Default::default()
            },
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }

    fn gameplay_logic(&mut self) {
        // Enhanced gameplay logic với collision detection thực tế hơn
        let mut entities_to_despawn = Vec::new();
        let mut scores_to_add = Vec::new();
        let mut new_pickups = Vec::new();
        let mut power_ups_collected = Vec::new();
        let mut damage_to_players = Vec::new();

        // Query để lấy tất cả player và pickup entities với physics bodies từ components

        // Collision detection sử dụng Rapier physics - tách thành các phần riêng để tránh borrow conflicts

        // 1. Player vs Pickups
        {
            let mut player_query = self.world.query::<(Entity, &TransformQ, &mut Player, &RigidBodyHandle)>();
            let mut pickup_query = self.world.query::<(Entity, &TransformQ, &Pickup, &RigidBodyHandle)>();

            for (player_entity, player_transform, player, player_rigid_body) in player_query.iter(&self.world) {
                for (pickup_entity, pickup_transform, pickup, pickup_rigid_body) in pickup_query.iter(&self.world) {
                    let player_pos = vector![player_transform.position[0], player_transform.position[1], player_transform.position[2]];
                    let pickup_pos = vector![pickup_transform.position[0], pickup_transform.position[1], pickup_transform.position[2]];
                    let distance = (player_pos - pickup_pos).magnitude();

                    if distance < 0.8 {
                        entities_to_despawn.push(pickup_entity);
                        scores_to_add.push((player.id.clone(), pickup.value));

                        let new_pos = [
                            (rand::random::<f32>() - 0.5) * 20.0,
                            1.0,
                            (rand::random::<f32>() - 0.5) * 20.0,
                        ];
                        new_pickups.push((new_pos, pickup.value + 5));

                        tracing::debug!(
                            "Pickup collected: player {} collected pickup worth {} at distance {}",
                            player.id, pickup.value, distance
                        );
                    }
                }
            }
        }

        // 2. Player vs Power-ups
        {
            let mut player_query = self.world.query::<(Entity, &TransformQ, &mut Player, &RigidBodyHandle)>();
            let mut powerup_query = self.world.query::<(Entity, &TransformQ, &PowerUp, &RigidBodyHandle)>();

            for (player_entity, player_transform, player, _player_rigid_body) in player_query.iter(&self.world) {
                for (power_up_entity, power_up_transform, power_up, _power_up_rigid_body) in powerup_query.iter(&self.world) {
                    let player_pos = vector![player_transform.position[0], player_transform.position[1], player_transform.position[2]];
                    let power_up_pos = vector![power_up_transform.position[0], power_up_transform.position[1], power_up_transform.position[2]];
                    let distance = (player_pos - power_up_pos).magnitude();

                    if distance < 0.7 {
                        entities_to_despawn.push(power_up_entity);
                        power_ups_collected.push((player.id.clone(), power_up.clone()));

                        tracing::debug!(
                            "Power-up collected: player {} collected {} power-up",
                            player.id, power_up.power_type
                        );
                    }
                }
            }
        }

        // 3. Player vs Enemies (combat damage)
        {
            let mut player_query = self.world.query::<(Entity, &TransformQ, &mut Player, &RigidBodyHandle)>();
            let mut enemy_query = self.world.query::<(Entity, &TransformQ, &Enemy, &RigidBodyHandle)>();

            for (player_entity, player_transform, player, _player_rigid_body) in player_query.iter(&self.world) {
                for (_enemy_entity, enemy_transform, enemy, _enemy_rigid_body) in enemy_query.iter(&self.world) {
                    let player_pos = vector![player_transform.position[0], player_transform.position[1], player_transform.position[2]];
                    let enemy_pos = vector![enemy_transform.position[0], enemy_transform.position[1], enemy_transform.position[2]];
                    let distance = (player_pos - enemy_pos).magnitude();

                    if distance < 1.0 {
                        if enemy.last_attack.elapsed() >= enemy.attack_cooldown {
                            damage_to_players.push((player.id.clone(), enemy.damage));

                            tracing::debug!(
                                "Enemy attack: {} enemy dealt {} damage to player {}",
                                enemy.enemy_type, enemy.damage, player.id
                            );
                        }
                    }
                }
            }
        }

        // 4. Player vs Obstacles (đơn giản là không thể đi qua) - collect changes first
        {
            let mut velocity_changes = Vec::new();

            let mut player_query = self.world.query::<(Entity, &TransformQ, &RigidBodyHandle)>();
            let mut obstacle_query = self.world.query::<(Entity, &TransformQ, &Obstacle, &RigidBodyHandle)>();

            for (player_entity, player_transform, _player_rigid_body) in player_query.iter(&self.world) {
                for (_obstacle_entity, obstacle_transform, obstacle, _obstacle_rigid_body) in obstacle_query.iter(&self.world) {
                    let player_pos = vector![player_transform.position[0], player_transform.position[1], player_transform.position[2]];
                    let obstacle_pos = vector![obstacle_transform.position[0], obstacle_transform.position[1], obstacle_transform.position[2]];
                    let distance = (player_pos - obstacle_pos).magnitude();

                    if distance < 1.5 {
                        let push_direction = (player_pos - obstacle_pos).normalize();
                        let push_force = 5.0;

                        velocity_changes.push((player_entity, push_direction.x * push_force, push_direction.z * push_force));
                    }
                }
            }

            // Apply velocity changes after queries
            for (player_entity, vel_x, vel_z) in velocity_changes {
                if let Some(mut velocity) = self.world.get_mut::<VelocityQ>(player_entity) {
                    velocity.velocity[0] += vel_x;
                    velocity.velocity[2] += vel_z;
                }
            }
        }

        // 5. Enemy AI - đơn giản di chuyển về phía player gần nhất - collect changes first
        {
            let mut velocity_changes = Vec::new();

            let mut enemy_query = self.world.query::<(Entity, &TransformQ, &Enemy, &RigidBodyHandle)>();
            let mut player_query = self.world.query::<(Entity, &TransformQ, &Player, &RigidBodyHandle)>();

            // Tạo list các players để tìm target
            let players: Vec<_> = player_query.iter(&self.world)
                .map(|(_, transform, player, _)| (player.id.clone(), transform.position))
                .collect();

            for (enemy_entity, enemy_transform, enemy, _enemy_rigid_body) in enemy_query.iter(&self.world) {
                // Tìm player gần nhất
                let mut nearest_player: Option<(String, [f32; 3])> = None;
                let mut nearest_distance = f32::INFINITY;

                for (player_id, player_pos) in &players {
                    let enemy_pos = vector![enemy_transform.position[0], enemy_transform.position[1], enemy_transform.position[2]];
                    let player_pos_vec = vector![player_pos[0], player_pos[1], player_pos[2]];
                    let distance = (enemy_pos - player_pos_vec).magnitude();

                    if distance < nearest_distance {
                        nearest_distance = distance;
                        nearest_player = Some((player_id.clone(), *player_pos));
                    }
                }

                // Tính toán velocity mới nếu tìm thấy player gần
                if let Some((_player_id, player_pos)) = nearest_player {
                    if nearest_distance > 2.0 {
                        let enemy_pos = vector![enemy_transform.position[0], enemy_transform.position[1], enemy_transform.position[2]];
                        let player_pos_vec = vector![player_pos[0], player_pos[1], player_pos[2]];
                        let direction = (player_pos_vec - enemy_pos).normalize();

                        velocity_changes.push((enemy_entity, direction.x * enemy.speed, direction.z * enemy.speed));
                    }
                }
            }

            // Apply velocity changes after queries
            for (enemy_entity, vel_x, vel_z) in velocity_changes {
                if let Some(mut velocity) = self.world.get_mut::<VelocityQ>(enemy_entity) {
                    velocity.velocity[0] = vel_x;
                    velocity.velocity[2] = vel_z;
                }
            }
        }

        // Second pass: apply changes

        // 1. Update scores từ pickups
        for (player_id, score_to_add) in scores_to_add {
            if let Some(player_entity) = self.world.resource::<PlayerEntityMap>().map.get(&player_id) {
                if let Some(mut player) = self.world.get_mut::<Player>(*player_entity) {
                    player.score += score_to_add;
                    tracing::debug!("Player {} score increased by {} (total: {})", player_id, score_to_add, player.score);
                }
            }
        }

        // 2. Apply damage từ enemies
        for (player_id, damage) in damage_to_players {
            if let Some(player_entity) = self.world.resource::<PlayerEntityMap>().map.get(&player_id) {
                if let Some(player) = self.world.get_mut::<Player>(*player_entity) {
                    // Trong MVP đơn giản, chỉ log damage (có thể thêm health system sau)
                    tracing::debug!("Player {} took {} damage", player_id, damage);
                }
            }
        }

        // 3. Apply power-up effects (MVP đơn giản - chỉ log)
        for (player_id, power_up) in power_ups_collected {
            tracing::debug!("Player {} activated {} power-up for {} seconds",
                player_id, power_up.power_type, power_up.duration.as_secs());
        }

        // 4. Update enemy attack timers
        let current_time = Instant::now();
        let mut enemy_query = self.world.query::<&mut Enemy>();
        for mut enemy in enemy_query.iter_mut(&mut self.world) {
            if current_time.duration_since(enemy.last_attack) >= enemy.attack_cooldown {
                // Reset attack timer để sẵn sàng attack tiếp
                enemy.last_attack = current_time;
            }
        }

        // Despawn collected entities
        for entity in entities_to_despawn {
            self.spatial_grid.remove_entity(entity);
            self.world.despawn(entity);
        }

        // Spawn new pickups
        for (pos, value) in new_pickups {
            self.add_pickup(pos, value);
        }
    }

    fn cleanup(&mut self) {
        // Cleanup entities với lifetime hết
        let mut to_despawn = Vec::new();
        let mut query = self.world.query::<(Entity, &Lifetime)>();
        for (entity, lifetime) in query.iter(&self.world) {
            if lifetime.remaining <= Duration::from_secs(0) {
                to_despawn.push(entity);
            }
        }

        for entity in to_despawn {
            self.spatial_grid.remove_entity(entity);
            self.world.despawn(entity);
        }

        // Update lifetime cho các entities còn sống
        let mut lifetime_query = self.world.query::<&mut Lifetime>();
        for mut lifetime in lifetime_query.iter_mut(&mut self.world) {
            lifetime.remaining = lifetime.remaining.saturating_sub(self.tick_rate);
        }
    }

    /// Update spatial grid với vị trí hiện tại của tất cả entities
    fn update_spatial_grid(&mut self) {
        let mut query = self.world.query::<(Entity, &TransformQ)>();
        for (entity, transform) in query.iter(&self.world) {
            // Update position in spatial grid if entity is already tracked
            if self.spatial_grid.entity_positions.contains_key(&entity) {
                self.spatial_grid.update_entity_position(entity, transform.position);
            }
        }
    }

    /// Update AOI for specific player
    fn update_player_aoi_grid(&mut self, player_id: &str) {
        // Update AOI for specific player
        if let Some(player_aoi) = self.player_aois.get_mut(player_id) {
            let current_tick = self.world.resource::<TickCount>().0;
            if current_tick - player_aoi.last_update_tick >= 10 { // Update every 10 ticks
                if let Some(player_entity) = self.world.resource::<PlayerEntityMap>().map.get(player_id) {
                    if let Some(transform) = self.world.get::<TransformQ>(*player_entity) {
                        player_aoi.visible_cells = self.spatial_grid.get_player_aoi_cells(transform.position);
                        player_aoi.last_update_tick = current_tick;
                    }
                }
            }
        }
    }

    pub fn create_snapshot(&mut self) -> GameSnapshot {
        let mut entities = Vec::new();

        let mut query = self.world.query::<(Entity, &TransformQ, Option<&VelocityQ>, Option<&Player>, Option<&Pickup>, Option<&Obstacle>, Option<&PowerUp>, Option<&Enemy>)>();
        for (entity, transform, velocity, player, pickup, obstacle, power_up, enemy) in query.iter(&self.world) {
            entities.push(EntitySnapshot {
                id: entity.index(),
                transform: transform.clone(),
                velocity: velocity.cloned(),
                player: player.cloned(),
                pickup: pickup.cloned(),
                obstacle: obstacle.cloned(),
                power_up: power_up.cloned(),
                enemy: enemy.map(|e| EnemySnapshot {
                    enemy_type: e.enemy_type.clone(),
                    damage: e.damage,
                    speed: e.speed,
                }),
            });
        }

        let spectators = self.get_spectator_snapshots();
        GameSnapshot {
            tick: self.current_tick,
            entities,
            chat_messages: self.get_recent_chat_messages(20),
            spectators,
        }
    }

    /// Lấy vị trí của player từ player_id
    pub fn get_player_position(&mut self, player_id: &str) -> Option<[f32; 3]> {
        let mut query = self.world.query::<(&Player, &TransformQ)>();
        for (player, transform) in query.iter(&self.world) {
            if player.id == player_id {
                return Some(transform.position);
            }
        }
        None
    }

    /// Lấy view distance của player từ player_id
    pub fn get_player_view_distance(&mut self, player_id: &str) -> Option<f32> {
        let mut query = self.world.query::<&Player>();
        for player in query.iter(&self.world) {
            if player.id == player_id {
                return Some(player.view_distance);
            }
        }
        None
    }

    pub fn add_player(&mut self, player_id: String) -> Entity {
        // Add to physics world first
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 5.0, 0.0])
            .build();
        let collider = ColliderBuilder::ball(0.5).build();

        let body_handle = self.bodies.insert(rigid_body);
        self.colliders.insert_with_parent(collider, body_handle, &mut self.bodies);

        // Create entity with components
        let entity = self.world.spawn((
            TransformQ {
                position: [0.0, 5.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            VelocityQ {
                velocity: [0.0, 0.0, 0.0],
                angular_velocity: [0.0, 0.0, 0.0],
            },
            Player {
                id: player_id.clone(),
                score: 0,
                view_distance: 50.0, // Default AOI radius
                last_position: [0.0, 5.0, 0.0], // Initial position
            },
            RigidBodyHandle {
                handle: body_handle,
            },
        ));

        let entity_id = entity.id();

        // Register vào PlayerEntityMap
        if let Some(mut player_map) = self.world.get_resource_mut::<PlayerEntityMap>() {
            player_map.map.insert(player_id, entity_id);
        }

        // Add to spatial grid
        self.spatial_grid.add_entity(entity_id, [0.0, 5.0, 0.0]);

        entity_id
    }

    /// Add a spectator to the game world
    pub fn add_spectator(&mut self, spectator_id: String, camera_mode: SpectatorCameraMode) -> Entity {
        // Create spectator entity without physics body (spectators don't interact with physics)
        let entity = self.world.spawn((
            TransformQ {
                position: [0.0, 10.0, 0.0], // Start above the game area
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            Spectator {
                id: spectator_id.clone(),
                target_player_id: None,
                camera_mode,
                view_distance: 100.0, // Larger view distance for spectators
                last_position: [0.0, 10.0, 0.0],
            },
        ));

        let entity_id = entity.id();

        // Add spectator to spatial grid (they still need to be tracked for AOI)
        self.spatial_grid.add_entity(entity_id, [0.0, 10.0, 0.0]);

        entity_id
    }

    pub fn add_pickup(&mut self, position: [f32; 3], value: u32) -> Entity {
        // Add to physics first
        let rigid_body = RigidBodyBuilder::fixed()
            .translation(vector![position[0], position[1], position[2]])
            .build();
        let collider = ColliderBuilder::ball(0.3).build();

        let body_handle = self.bodies.insert(rigid_body);
        self.colliders.insert_with_parent(collider, body_handle, &mut self.bodies);

        // Create entity with components
        let entity = self.world.spawn((
            TransformQ {
                position,
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            Pickup { value },
            Lifetime {
                remaining: Duration::from_secs(30), // Pickup tồn tại 30s
            },
            RigidBodyHandle {
                handle: body_handle,
            },
        ));

        let entity_id = entity.id();

        // Add pickup to spatial grid
        self.spatial_grid.add_entity(entity_id, position);

        entity_id
    }

    pub fn add_obstacle(&mut self, position: [f32; 3], obstacle_type: String) -> Entity {
        // Add to physics first
        let rigid_body = RigidBodyBuilder::fixed()
            .translation(vector![position[0], position[1], position[2]])
            .build();

        // Collider size phụ thuộc loại obstacle
        let collider = match obstacle_type.as_str() {
            "wall" => ColliderBuilder::cuboid(2.0, 1.0, 0.5).build(),
            "spike" => ColliderBuilder::ball(0.5).build(),
            "moving_platform" => ColliderBuilder::cuboid(3.0, 0.3, 2.0).build(),
            _ => ColliderBuilder::cuboid(1.0, 1.0, 1.0).build(),
        };

        let body_handle = self.bodies.insert(rigid_body);
        self.colliders.insert_with_parent(collider, body_handle, &mut self.bodies);

        // Create entity with components
        let entity = self.world.spawn((
            TransformQ {
                position,
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            Obstacle {
                obstacle_type: obstacle_type.clone(),
            },
            RigidBodyHandle {
                handle: body_handle,
            },
        ));

        let entity_id = entity.id();

        // Add obstacle to spatial grid
        self.spatial_grid.add_entity(entity_id, position);

        entity_id
    }

    pub fn add_power_up(&mut self, position: [f32; 3], power_type: String, duration_secs: u64, value: u32) -> Entity {
        // Add to physics first
        let rigid_body = RigidBodyBuilder::fixed()
            .translation(vector![position[0], position[1], position[2]])
            .build();
        let collider = ColliderBuilder::ball(0.4).build();

        let body_handle = self.bodies.insert(rigid_body);
        self.colliders.insert_with_parent(collider, body_handle, &mut self.bodies);

        // Create entity with components
        let entity = self.world.spawn((
            TransformQ {
                position,
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            PowerUp {
                power_type: power_type.clone(),
                duration: Duration::from_secs(duration_secs),
                value,
            },
            Lifetime {
                remaining: Duration::from_secs(60), // Power-up tồn tại 60s
            },
            RigidBodyHandle {
                handle: body_handle,
            },
        ));

        let entity_id = entity.id();

        // Add power-up to spatial grid
        self.spatial_grid.add_entity(entity_id, position);

        entity_id
    }

    pub fn add_enemy(&mut self, position: [f32; 3], enemy_type: String) -> Entity {
        // Add to physics first
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![position[0], position[1], position[2]])
            .build();

        let collider = match enemy_type.as_str() {
            "basic" => ColliderBuilder::ball(0.6).build(),
            "fast" => ColliderBuilder::ball(0.4).build(),
            "tank" => ColliderBuilder::ball(0.8).build(),
            _ => ColliderBuilder::ball(0.6).build(),
        };

        let body_handle = self.bodies.insert(rigid_body);
        self.colliders.insert_with_parent(collider, body_handle, &mut self.bodies);

        // Enemy stats phụ thuộc loại
        let (damage, speed, attack_cooldown) = match enemy_type.as_str() {
            "basic" => (10, 2.0, Duration::from_secs(2)),
            "fast" => (5, 5.0, Duration::from_secs(1)),
            "tank" => (20, 1.0, Duration::from_secs(3)),
            _ => (10, 2.0, Duration::from_secs(2)),
        };

        // Create entity with components
        let entity = self.world.spawn((
            TransformQ {
                position,
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            VelocityQ {
                velocity: [0.0, 0.0, 0.0],
                angular_velocity: [0.0, 0.0, 0.0],
            },
            Enemy {
                enemy_type: enemy_type.clone(),
                damage,
                speed,
                last_attack: Instant::now(),
                attack_cooldown,
            },
            RigidBodyHandle {
                handle: body_handle,
            },
        ));

        let entity_id = entity.id();

        // Add enemy to spatial grid
        self.spatial_grid.add_entity(entity_id, position);

        entity_id
    }

    /// Endless Runner specific gameplay logic
    pub fn update_endless_runner(&mut self, delta_time: Duration) {
        // Auto-run forward movement for all players
        let mut player_query = self.world.query::<(&mut TransformQ, &mut Player)>();
        for (mut transform, mut player) in player_query.iter_mut(&mut self.world) {
            let run_speed = 12.0; // Base running speed for endless runner
            transform.position[2] += run_speed * delta_time.as_secs_f32();

            // Update player score based on distance traveled
            let distance_traveled = transform.position[2] - player.last_position[2];
            if distance_traveled > 0.0 {
                player.score += (distance_traveled * 10.0) as u32; // Score per unit distance
                player.last_position = transform.position;
            }
        }

        // Procedural obstacle generation for endless runner
        self.generate_endless_runner_obstacles();

        // Lane-based movement constraints (keep players in their lanes)
        self.update_lane_positions();
    }

    /// Generate obstacles ahead of players for endless runner
    fn generate_endless_runner_obstacles(&mut self) {
        let mut player_positions = Vec::new();
        let mut player_query = self.world.query::<(&TransformQ, &Player)>();
        for (transform, _) in player_query.iter(&self.world) {
            player_positions.push(transform.position[2]);
        }

        for player_z in player_positions {
            // Generate obstacles 60-100 units ahead (farther for endless runner)
            if player_z % 25.0 < 0.1 { // Every 25 units for more spaced obstacles
                let obstacle_z = player_z + 60.0 + (rand::random::<f32>() * 40.0);
                let lane = rand::random::<usize>() % 3;
                let lanes = [-3.0, 0.0, 3.0]; // Wider lanes for 3D

                // Random obstacle type for variety
                let obstacle_types = ["wall", "spike", "moving_platform"];
                let obstacle_type = obstacle_types[rand::random::<usize>() % obstacle_types.len()];

                self.add_obstacle(
                    [lanes[lane], 0.5, obstacle_z],
                    obstacle_type.to_string()
                );
            }

            // Occasionally spawn power-ups
            if player_z % 50.0 < 0.1 && rand::random::<f32>() < 0.3 { // 30% chance every 50 units
                let powerup_z = player_z + 70.0 + (rand::random::<f32>() * 30.0);
                let lane = rand::random::<usize>() % 3;
                let lanes = [-3.0, 0.0, 3.0];

                let power_types = ["speed_boost", "jump_boost", "invincibility"];
                let power_type = power_types[rand::random::<usize>() % power_types.len()];

                self.add_power_up(
                    [lanes[lane], 2.0, powerup_z],
                    power_type.to_string(),
                    10, // 10 seconds duration
                    100 // 100 points value
                );
            }
        }
    }

    /// Keep players in their lanes (snap to lane positions)
    fn update_lane_positions(&mut self) {
        let mut query = self.world.query::<(&mut TransformQ, &mut Player)>();
        for (mut transform, _) in query.iter_mut(&mut self.world) {
            // Snap to lane positions (x-axis) for endless runner
            let lanes = [-3.0, 0.0, 3.0];
            let closest_lane = lanes.iter()
                .min_by(|a, b| (transform.position[0] - **a).abs().partial_cmp(&(transform.position[0] - **b).abs()).unwrap())
                .unwrap();
            transform.position[0] = *closest_lane;
        }
    }

    /// Add endless runner specific pickup (coins/gems)
    pub fn add_endless_runner_pickup(&mut self, position: [f32; 3], value: u32) -> Entity {
        // Add to physics first
        let rigid_body = RigidBodyBuilder::fixed()
            .translation(vector![position[0], position[1], position[2]])
            .build();
        let collider = ColliderBuilder::ball(0.4).build();

        let body_handle = self.bodies.insert(rigid_body);
        self.colliders.insert_with_parent(collider, body_handle, &mut self.bodies);

        // Create entity with components
        let entity = self.world.spawn((
            TransformQ {
                position,
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            Pickup { value },
            Lifetime {
                remaining: Duration::from_secs(30), // Pickup tồn tại 30s
            },
            RigidBodyHandle {
                handle: body_handle,
            },
        ));

        let entity_id = entity.id();

        // Add endless runner pickup to spatial grid
        self.spatial_grid.add_entity(entity_id, position);

        entity_id
    }
}

/// Resources cho ECS
#[derive(Resource, Default)]
pub struct InputBuffers {
    pub buffers: std::collections::HashMap<String, InputBuffer>,
}

#[derive(Resource, Default)]
pub struct TickCount(pub u64);

#[derive(Resource, Default)]
pub struct PlayerEntityMap {
    pub map: std::collections::HashMap<String, Entity>,
}

/// Spawn một số entities để test với gameplay thực tế hơn
pub fn spawn_test_entities(world: &mut GameWorld) {
    // Spawn player ở vị trí trung tâm
    world.add_player("player_1".to_string());

    // Spawn nhiều pickups ở vị trí random với giá trị khác nhau
    for _ in 0..10 {
        let x = (rand::random::<f32>() - 0.5) * 25.0;
        let z = (rand::random::<f32>() - 0.5) * 25.0;
        let value = (rand::random::<f32>() * 15.0 + 5.0) as u32; // Giá trị từ 5-20
        world.add_pickup([x, 1.0, z], value);
    }

    // Spawn obstacles để làm gameplay thú vị hơn
    for i in 0..6 {
        let x = (i as f32 - 3.0) * 4.0;
        let z = (rand::random::<f32>() - 0.5) * 20.0;
        world.add_obstacle([x, 0.5, z], "wall".to_string());
    }

    // Spawn một số power-ups đặc biệt
    for i in 0..3 {
        let x = (i as f32 - 1.0) * 8.0;
        let z = 0.0;
        world.add_power_up([x, 2.0, z], "speed_boost".to_string(), 10, 50);
    }

    // Spawn enemies để test AI và combat
    for i in 0..4 {
        let x = (i as f32 - 2.0) * 6.0;
        let z = (rand::random::<f32>() - 0.5) * 15.0 + 10.0; // Spawn xa hơn để tránh player ban đầu
        let enemy_type = match i % 3 {
            0 => "basic",
            1 => "fast",
            _ => "tank",
        };
        world.add_enemy([x, 1.0, z], enemy_type.to_string());
    }
}
