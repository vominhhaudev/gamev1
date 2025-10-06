/// Background job system for maintenance and cleanup tasks
/// Handles periodic cleanup, leaderboard updates, and system maintenance

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use crate::persistence::{PersistenceState, cleanup_old_data, create_persistence_state};

/// Background job types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    /// Clean up old game data
    CleanupOldData {
        older_than_days: u32,
    },
    /// Update leaderboard rankings
    UpdateLeaderboard {
        game_mode: String,
        season: String,
    },
    /// Generate daily user statistics
    GenerateDailyStats {
        date: String, // YYYY-MM-DD
    },
    /// Process user achievements
    ProcessAchievements {
        user_id: String,
    },
    /// Maintenance tasks
    Maintenance {
        task_type: String,
    },
}

/// Job execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub job_type: JobType,
    pub status: JobStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
}

/// Job execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Job queue and execution system
pub struct JobSystem {
    pub persistence_state: PersistenceState,
    pub active_jobs: RwLock<HashMap<String, JobResult>>,
    pub job_history: RwLock<Vec<JobResult>>,
    pub max_concurrent_jobs: usize,
}

impl JobSystem {
    /// Create new job system
    pub fn new(persistence_state: PersistenceState) -> Self {
        Self {
            persistence_state,
            active_jobs: RwLock::new(HashMap::new()),
            job_history: RwLock::new(Vec::new()),
            max_concurrent_jobs: 5,
        }
    }

    /// Start the job scheduler
    pub async fn start_scheduler(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Starting background job scheduler");

        // Cleanup job - runs every hour
        let persistence_state = self.persistence_state.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // Every hour
            loop {
                interval.tick().await;

                let job = JobType::CleanupOldData {
                    older_than_days: 30,
                };

                // Create a minimal job system for this task
                let job_system = JobSystem::new(persistence_state.clone());
                if let Err(e) = job_system.execute_job(job).await {
                    tracing::error!("Cleanup job failed: {:?}", e);
                }
            }
        });

        // Leaderboard update job - runs every 15 minutes
        let persistence_state = self.persistence_state.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(900)); // Every 15 minutes
            loop {
                interval.tick().await;

                // Update leaderboards for all game modes
                for game_mode in ["deathmatch", "team_deathmatch", "capture_the_flag"] {
                    let job = JobType::UpdateLeaderboard {
                        game_mode: game_mode.to_string(),
                        season: "season_1".to_string(),
                    };

                    // Create a minimal job system for this task
                    let job_system = JobSystem::new(persistence_state.clone());
                    if let Err(e) = job_system.execute_job(job).await {
                        tracing::error!("Leaderboard update job failed for {}: {:?}", game_mode, e);
                    }

                    // Small delay between game modes
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });

        // Daily stats generation - runs at midnight
        let persistence_state = self.persistence_state.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(86400)); // Every 24 hours
            loop {
                interval.tick().await;

                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let job = JobType::GenerateDailyStats {
                    date: today,
                };

                // Create a minimal job system for this task
                let job_system = JobSystem::new(persistence_state.clone());
                if let Err(e) = job_system.execute_job(job).await {
                    tracing::error!("Daily stats generation failed: {:?}", e);
                }
            }
        });

        Ok(())
    }

    /// Execute a single job
    pub async fn execute_job(&self, job_type: JobType) -> Result<JobResult, Box<dyn std::error::Error + Send + Sync>> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let started_at = Utc::now();

        // Check if we can run more jobs
        {
            let active_jobs = self.active_jobs.read().await;
            if active_jobs.len() >= self.max_concurrent_jobs {
                return Err("Too many concurrent jobs running".into());
            }
        }

        // Create job result
        let mut job_result = JobResult {
            job_id: job_id.clone(),
            job_type: job_type.clone(),
            status: JobStatus::Running,
            started_at,
            completed_at: None,
            duration_ms: None,
            error_message: None,
            metadata: serde_json::json!({}),
        };

        // Add to active jobs
        {
            let mut active_jobs = self.active_jobs.write().await;
            active_jobs.insert(job_id.clone(), job_result.clone());
        }

        // Execute the job
        let execution_result = self.execute_job_internal(&job_type).await;

        // Update job result
        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        job_result.status = if execution_result.is_ok() { JobStatus::Completed } else { JobStatus::Failed };
        job_result.completed_at = Some(completed_at);
        job_result.duration_ms = Some(duration_ms);

        if let Err(e) = &execution_result {
            job_result.error_message = Some(e.to_string());
            job_result.metadata = serde_json::json!({
                "error_details": e.to_string()
            });
        } else if let Ok(metadata) = &execution_result {
            job_result.metadata = metadata.clone();
        }

        // Remove from active jobs and add to history
        {
            let mut active_jobs = self.active_jobs.write().await;
            active_jobs.remove(&job_id);
        }

        {
            let mut history = self.job_history.write().await;
            history.push(job_result.clone());

            // Keep only last 1000 jobs in history
            if history.len() > 1000 {
                history.drain(0..100);
            }
        }

        tracing::info!(
            "Job {} completed in {}ms: {:?}",
            job_id,
            duration_ms,
            job_result.status
        );

        execution_result.map(|_| job_result)
    }

    /// Internal job execution logic
    async fn execute_job_internal(&self, job_type: &JobType) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        match job_type {
            JobType::CleanupOldData { older_than_days } => {
                let records_cleaned = cleanup_old_data(&self.persistence_state, *older_than_days).await?;
                Ok(serde_json::json!({
                    "records_cleaned": records_cleaned,
                    "older_than_days": older_than_days
                }))
            }
            JobType::UpdateLeaderboard { game_mode, season } => {
                // Mock leaderboard update
                tracing::info!("Updating leaderboard for {} in season {}", game_mode, season);
                tokio::time::sleep(Duration::from_secs(2)).await; // Simulate work

                Ok(serde_json::json!({
                    "game_mode": game_mode,
                    "season": season,
                    "entries_updated": 150
                }))
            }
            JobType::GenerateDailyStats { date } => {
                // Mock daily stats generation
                tracing::info!("Generating daily stats for date {}", date);
                tokio::time::sleep(Duration::from_secs(5)).await; // Simulate work

                Ok(serde_json::json!({
                    "date": date,
                    "users_processed": 500,
                    "stats_records_created": 500
                }))
            }
            JobType::ProcessAchievements { user_id } => {
                // Mock achievement processing
                tracing::info!("Processing achievements for user {}", user_id);
                tokio::time::sleep(Duration::from_secs(1)).await; // Simulate work

                Ok(serde_json::json!({
                    "user_id": user_id,
                    "achievements_unlocked": 2,
                    "points_awarded": 150
                }))
            }
            JobType::Maintenance { task_type } => {
                // Mock maintenance task
                tracing::info!("Running maintenance task: {}", task_type);
                tokio::time::sleep(Duration::from_secs(3)).await; // Simulate work

                Ok(serde_json::json!({
                    "task_type": task_type,
                    "operations_completed": 10
                }))
            }
        }
    }

    /// Get current job statistics
    pub async fn get_job_stats(&self) -> JobStats {
        let active_jobs = self.active_jobs.read().await;
        let history = self.job_history.read().await;

        let mut status_counts = HashMap::new();
        let mut type_counts = HashMap::new();
        let mut total_duration: u64 = 0;
        let mut completed_count = 0;

        for job in history.iter().rev().take(1000) { // Last 1000 jobs
            *status_counts.entry(job.status.clone()).or_insert(0) += 1;
            *type_counts.entry(format!("{:?}", job.job_type)).or_insert(0) += 1;

            if job.status == JobStatus::Completed {
                if let Some(duration) = job.duration_ms {
                    total_duration += duration;
                    completed_count += 1;
                }
            }
        }

        let avg_duration_ms = if completed_count > 0 {
            total_duration / completed_count
        } else {
            0
        };

        JobStats {
            active_jobs: active_jobs.len(),
            total_jobs_today: history.iter()
                .filter(|j| j.started_at.date_naive() == Utc::now().date_naive())
                .count(),
            completed_jobs: *status_counts.get(&JobStatus::Completed).unwrap_or(&0),
            failed_jobs: *status_counts.get(&JobStatus::Failed).unwrap_or(&0),
            avg_execution_time_ms: avg_duration_ms,
            job_type_distribution: type_counts,
        }
    }
}

/// Job statistics
#[derive(Debug, Serialize)]
pub struct JobStats {
    pub active_jobs: usize,
    pub total_jobs_today: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
    pub avg_execution_time_ms: u64,
    pub job_type_distribution: HashMap<String, usize>,
}

/// Manual job execution for API endpoints
pub async fn execute_manual_job(
    job_system: &JobSystem,
    job_type: JobType,
) -> Result<JobResult, Box<dyn std::error::Error + Send + Sync>> {
    job_system.execute_job(job_type).await
}

/// Get job history with filtering
pub async fn get_job_history(
    job_system: &JobSystem,
    limit: Option<usize>,
    status_filter: Option<JobStatus>,
) -> Result<Vec<JobResult>, Box<dyn std::error::Error + Send + Sync>> {
    let history = job_system.job_history.read().await;
    let mut filtered = history.clone();

    // Apply status filter
    if let Some(status) = status_filter {
        filtered.retain(|job| job.status == status);
    }

    // Apply limit
    if let Some(limit) = limit {
        filtered.truncate(limit);
    }

    Ok(filtered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_type_creation() {
        let cleanup_job = JobType::CleanupOldData {
            older_than_days: 30,
        };

        match cleanup_job {
            JobType::CleanupOldData { older_than_days } => {
                assert_eq!(older_than_days, 30);
            }
            _ => panic!("Wrong job type"),
        }
    }

    #[test]
    fn test_job_status() {
        assert_eq!(JobStatus::Pending, JobStatus::Pending);
        assert_ne!(JobStatus::Pending, JobStatus::Running);
    }

    #[tokio::test]
    async fn test_job_system_creation() {
        let persistence_state = create_persistence_state("http://localhost:8090".to_string());
        let job_system = JobSystem::new(persistence_state);

        assert_eq!(job_system.max_concurrent_jobs, 5);

        let stats = job_system.get_job_stats().await;
        assert_eq!(stats.active_jobs, 0);
        assert_eq!(stats.total_jobs_today, 0);
    }

    #[test]
    fn test_job_result_creation() {
        let job_result = JobResult {
            job_id: "test_job".to_string(),
            job_type: JobType::CleanupOldData { older_than_days: 30 },
            status: JobStatus::Completed,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration_ms: Some(1500),
            error_message: None,
            metadata: serde_json::json!({}),
        };

        assert_eq!(job_result.job_id, "test_job");
        assert_eq!(job_result.status, JobStatus::Completed);
        assert_eq!(job_result.duration_ms, Some(1500));
    }
}
