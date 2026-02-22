use tracing::{info, warn};

use super::super::manager::ConnectionEvent;
use super::PrioritySubscriptionManager;

impl PrioritySubscriptionManager {
    pub(super) fn log_connection_event(&self, event: &ConnectionEvent) {
        match event {
            ConnectionEvent::Connected { connection_id } => {
                info!(connection_id = *connection_id, "Connection established");
            }
            ConnectionEvent::Disconnected {
                connection_id,
                reason,
            } => {
                warn!(
                    connection_id = *connection_id,
                    reason = %reason,
                    "Connection lost"
                );
            }
            ConnectionEvent::ShardUnhealthy { shard_id } => {
                warn!(shard_id = *shard_id, "Shard became unhealthy");
            }
            ConnectionEvent::ShardRecovered { shard_id } => {
                info!(shard_id = *shard_id, "Shard recovered");
            }
        }
    }
}
