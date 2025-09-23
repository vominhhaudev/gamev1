use std::time::Duration;

use common_net::metrics;

/// Helper ghi nh?n metric cho pipeline snapshot/delta.
#[derive(Debug, Default)]
pub struct SnapshotRecorder;

impl SnapshotRecorder {
    pub fn record_broadcast(&self, encode_duration: Duration) {
        let metrics = metrics::snapshot_metrics();
        metrics.inc_snapshots_broadcast();
        metrics.observe_encode_seconds(encode_duration.as_secs_f64());
    }
}
