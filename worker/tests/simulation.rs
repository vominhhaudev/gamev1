use worker::simulation::{SimulationWorld, PlayerInput, GameSnapshot};
use std::time::Duration;

#[tokio::test]
async fn simulation_world_creation() {
    let world = SimulationWorld::new();
    assert_eq!(world.tick_count, 0);
    assert_eq!(world.entities.len(), 0);
}

#[tokio::test]
async fn simulation_step() {
    let mut world = SimulationWorld::new();
    world.step(Duration::from_millis(16)); // 60 FPS step
    assert_eq!(world.tick_count, 1);
    assert_eq!(world.entities.len(), 1); // Should have added one entity
}

#[tokio::test]
async fn snapshot_creation() {
    let mut world = SimulationWorld::new();

    // Step once to add entity
    world.step(Duration::from_millis(16));

    let snapshot = world.create_snapshot();
    assert_eq!(snapshot.tick, 1);
    assert_eq!(snapshot.entities.len(), 1); // One entity added
}

// TODO: Add input buffer tests when InputBuffer is implemented
