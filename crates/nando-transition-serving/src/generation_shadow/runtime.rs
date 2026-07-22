use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::sync::{Arc, OnceLock};
use std::thread;

use super::loader::load_generation_shadow_snapshot_v3;
use super::telemetry::GenerationShadowTelemetryV3;
use super::watcher::run_generation_shadow_watcher_v3;
use super::worker::{run_generation_shadow_worker_v3, GenerationShadowWorkItemV3};
use super::{
    GenerationShadowConfigV3, GenerationShadowRegistryV3, GenerationShadowRequestV3,
    GenerationShadowStatusV3, GenerationShadowSubmitVerdictV3,
};

pub struct GenerationShadowRuntimeV3 {
    config: GenerationShadowConfigV3,
    registry: Arc<GenerationShadowRegistryV3>,
    telemetry: Arc<GenerationShadowTelemetryV3>,
    sender: OnceLock<SyncSender<GenerationShadowWorkItemV3>>,
    started: AtomicBool,
}

impl GenerationShadowRuntimeV3 {
    pub fn new(config: GenerationShadowConfigV3) -> Result<Self, &'static str> {
        config.validate()?;
        let enabled = config.enabled;
        Ok(Self {
            config,
            registry: Arc::new(GenerationShadowRegistryV3::default()),
            telemetry: Arc::new(GenerationShadowTelemetryV3::new(enabled)),
            sender: OnceLock::new(),
            started: AtomicBool::new(false),
        })
    }

    pub fn start_after_http_bind(self: &Arc<Self>) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }
        if self
            .started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Ok(());
        }
        let (sender, receiver) = sync_channel(self.config.queue_capacity);
        self.sender
            .set(sender)
            .map_err(|_| "generation_shadow_sender_already_set".to_owned())?;

        let telemetry = Arc::clone(&self.telemetry);
        thread::Builder::new()
            .name("nando-generation-shadow-worker".to_owned())
            .spawn(move || run_generation_shadow_worker_v3(receiver, telemetry))
            .map_err(|error| format!("generation_shadow_worker_spawn:{error}"))?;

        let runtime = Arc::clone(self);
        let config = self.config.clone();
        thread::Builder::new()
            .name("nando-generation-shadow-loader".to_owned())
            .spawn(move || run_generation_shadow_watcher_v3(runtime, config))
            .map(|_| ())
            .map_err(|error| format!("generation_shadow_loader_spawn:{error}"))
    }

    pub fn reconcile_once(&self) -> Result<bool, String> {
        if !self.config.enabled {
            return Ok(false);
        }
        self.telemetry.loading();
        match load_generation_shadow_snapshot_v3(&self.config) {
            Ok(Some(snapshot)) => {
                let update = self.registry.install(snapshot).map_err(|error| {
                    let message = format!("generation_shadow_registry:{error:?}");
                    self.telemetry.blocked(&message);
                    message
                })?;
                let pinned = self
                    .registry
                    .pin()
                    .map_err(|error| format!("generation_shadow_pin:{error:?}"))?
                    .ok_or_else(|| "generation_shadow_registry_empty".to_owned())?;
                let checkpoint = pinned.checkpoint();
                self.telemetry.ready(
                    checkpoint.generation().manifest().sequence(),
                    checkpoint.generation().manifest().generation_id_sha256(),
                    checkpoint.publish_sequence(),
                    checkpoint.checkpoint_sha256(),
                    pinned.capture_index_sha256(),
                );
                Ok(matches!(
                    update,
                    super::GenerationShadowRegistryUpdateV3::Installed
                ))
            }
            Ok(None) => {
                self.registry
                    .clear()
                    .map_err(|error| format!("generation_shadow_clear:{error:?}"))?;
                self.telemetry.empty();
                Ok(false)
            }
            Err(error) => {
                let mut message = format!("generation_shadow_load:{error:?}");
                if let Err(clear_error) = self.registry.clear() {
                    message.push_str(&format!(";generation_shadow_clear:{clear_error:?}"));
                }
                self.telemetry.blocked(&message);
                Err(message)
            }
        }
    }

    pub fn try_submit(
        &self,
        request: GenerationShadowRequestV3,
    ) -> GenerationShadowSubmitVerdictV3 {
        let verdict = if !self.config.enabled {
            GenerationShadowSubmitVerdictV3::CensoredDisabled
        } else if !self.started.load(Ordering::Acquire) {
            GenerationShadowSubmitVerdictV3::CensoredNotStarted
        } else {
            match (self.sender.get(), self.registry.pin()) {
                (Some(sender), Ok(Some(generation))) => {
                    match sender.try_send(GenerationShadowWorkItemV3::new(generation, request)) {
                        Ok(()) => GenerationShadowSubmitVerdictV3::Enqueued,
                        Err(TrySendError::Full(_)) => {
                            GenerationShadowSubmitVerdictV3::CensoredQueueFull
                        }
                        Err(TrySendError::Disconnected(_)) => {
                            GenerationShadowSubmitVerdictV3::CensoredDisconnected
                        }
                    }
                }
                (None, _) => GenerationShadowSubmitVerdictV3::CensoredNotStarted,
                (Some(_), Ok(None) | Err(_)) => {
                    GenerationShadowSubmitVerdictV3::CensoredNoGeneration
                }
            }
        };
        self.telemetry.observe_submit(verdict);
        verdict
    }

    pub fn observe_censored(
        &self,
        verdict: GenerationShadowSubmitVerdictV3,
    ) -> GenerationShadowSubmitVerdictV3 {
        debug_assert_ne!(verdict, GenerationShadowSubmitVerdictV3::Enqueued);
        self.telemetry.observe_submit(verdict);
        verdict
    }

    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.config.enabled
    }

    #[must_use]
    pub fn status(&self) -> GenerationShadowStatusV3 {
        self.telemetry.snapshot()
    }

    #[must_use]
    pub const fn registry(&self) -> &Arc<GenerationShadowRegistryV3> {
        &self.registry
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
