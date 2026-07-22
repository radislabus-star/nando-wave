use std::sync::mpsc::TrySendError;

use super::{TrafficShadowHandoffCountersV3, TrafficShadowHandoffVerdictV3};

impl TrafficShadowHandoffCountersV3 {
    pub fn observe<T>(
        &mut self,
        result: Result<(), TrySendError<T>>,
    ) -> TrafficShadowHandoffVerdictV3 {
        self.attempted = self.attempted.saturating_add(1);
        match result {
            Ok(()) => {
                self.enqueued = self.enqueued.saturating_add(1);
                TrafficShadowHandoffVerdictV3::Enqueued
            }
            Err(TrySendError::Full(_)) => {
                self.censored_queue_full = self.censored_queue_full.saturating_add(1);
                TrafficShadowHandoffVerdictV3::CensoredQueueFull
            }
            Err(TrySendError::Disconnected(_)) => {
                self.censored_disconnected = self.censored_disconnected.saturating_add(1);
                TrafficShadowHandoffVerdictV3::CensoredDisconnected
            }
        }
    }

    #[must_use]
    pub const fn attempted(&self) -> u64 {
        self.attempted
    }

    #[must_use]
    pub const fn accounted(&self) -> u64 {
        self.enqueued
            .saturating_add(self.censored_queue_full)
            .saturating_add(self.censored_disconnected)
    }

    #[must_use]
    pub const fn enqueued(&self) -> u64 {
        self.enqueued
    }

    #[must_use]
    pub const fn censored_queue_full(&self) -> u64 {
        self.censored_queue_full
    }

    #[must_use]
    pub const fn censored_disconnected(&self) -> u64 {
        self.censored_disconnected
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
