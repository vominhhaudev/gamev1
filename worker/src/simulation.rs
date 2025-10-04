
use bevy_ecs::prelude::*;
use rapier3d::prelude::*;
use rapier3d::geometry::DefaultBroadPhase;
use rapier3d::dynamics::{MultibodyJointSet, ImpulseJointSet};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing;

use crate::validation::{InputValidator, ValidationConfig, ValidationError};

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
        }
    }

    /// Main game loop với fixed timestep
    pub fn tick(&mut self) -> GameSnapshot {
        let now = std::time::Instant::now();
        self.accumulator += now - self.last_tick;
        self.last_tick = now;

        // Fixed timestep - chỉ tick khi đủ thời gian
        let mut ticks = 0;
        while self.accumulator >= self.tick_rate && ticks < 3 { // Max 3 ticks per frame
            self.fixed_update();
            self.accumulator -= self.tick_rate;
            ticks += 1;
        }

        // Luôn trả về snapshot mới nhất, tick count sẽ được cập nhật trong fixed_update
        self.create_snapshot()
    }

    /// Chạy simulation trong thời gian ngắn để test
    pub fn run_simulation_for_test(&mut self, duration_secs: f32) -> Vec<GameSnapshot> {
        let mut snapshots = Vec::new();
        let target_ticks = (duration_secs * 60.0) as u32; // 60 FPS

        // Đảm bảo có ít nhất một số entities để test
        if self.world.query::<&Player>().iter(&self.world).count() == 0 {
            self.add_player("test_player".to_string());
        }

        for i in 0..target_ticks {
            // Gọi fixed_update trực tiếp để đảm bảo tick count tăng
            self.fixed_update();
            let snapshot = self.create_snapshot();
            snapshots.push(snapshot);
        }

        snapshots
    }

    /// Get current snapshot without mutating state (for external use) - deprecated, use get_snapshot_for_player
    pub fn get_snapshot(&mut self) -> GameSnapshot {
        // Fallback to full snapshot for backward compatibility
        self.create_snapshot()
    }

    /// Get current snapshot for a specific player using AOI optimization
    pub fn get_snapshot_for_player(&mut self, player_id: &str) -> GameSnapshot {
        let player_position = self.get_player_position(player_id)
            .unwrap_or([0.0, 5.0, 0.0]);
        let view_distance = self.get_player_view_distance(player_id)
            .unwrap_or(50.0); // Default view distance

        // Create AOI-optimized snapshot
        let mut entities = Vec::new();
        let mut query = self.world.query::<(Entity, &TransformQ, Option<&Player>, Option<&Pickup>, Option<&Obstacle>, Option<&PowerUp>, Option<&Enemy>)>();

        for (entity, transform, player, pickup, obstacle, power_up, enemy) in query.iter(&self.world) {
            // Calculate distance from player to entity
            let entity_pos = vector![transform.position[0], transform.position[1], transform.position[2]];
            let player_pos_vec = vector![player_position[0], player_position[1], player_position[2]];
            let distance = (entity_pos - player_pos_vec).magnitude();

            // Include entity if within view distance or if it's the player themselves
            if distance <= view_distance || (player.is_some() && player.as_ref().unwrap().id == player_id) {
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

        let spectators = self.get_spectator_snapshots();
        GameSnapshot {
            tick: self.world.resource::<TickCount>().0,
            entities,
            chat_messages: self.get_recent_chat_messages(20),
            spectators,
        }
    }

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
        // Tăng tick count
        if let Some(mut tick_count) = self.world.get_resource_mut::<TickCount>() {
            tick_count.0 += 1;
        }

        // 1. Ingest và validate inputs
        self.ingest_inputs();

        // 2. Validate inputs (anti-cheat cơ bản)
        self.validate_inputs();

        // 3. Physics step
        self.physics_step();

        // 4. Gameplay logic
        self.gameplay_logic();

        // 5. Cleanup (lifetime, etc.)
        self.cleanup();

        // 6. Room cleanup
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

            for (player_entity, player_transform, mut player, player_rigid_body) in player_query.iter(&self.world) {
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

            for (player_entity, player_transform, mut player, _player_rigid_body) in player_query.iter(&self.world) {
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

            for (player_entity, player_transform, mut player, _player_rigid_body) in player_query.iter(&self.world) {
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
                if let Some(mut player) = self.world.get_mut::<Player>(*player_entity) {
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
            self.world.despawn(entity);
        }

        // Update lifetime cho các entities còn sống
        let mut lifetime_query = self.world.query::<&mut Lifetime>();
        for mut lifetime in lifetime_query.iter_mut(&mut self.world) {
            lifetime.remaining = lifetime.remaining.saturating_sub(self.tick_rate);
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
            tick: self.world.resource::<TickCount>().0,
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

        entity.id()
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

        entity.id()
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

        entity.id()
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

        entity.id()
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

        entity.id()
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
