use bevy::ecs::message::{Message, MessageReader};
use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait};

use crate::elf_game::moose::{MooseHuffEvent, MooseWindowSlamEvent};

/// Configure Linux environment variables for ALSA and sound servers before driver initialization.
/// Setting AUDIODEV to "default" ensures ALSA uses software mixing (PipeWire/PulseAudio/dmix)
/// rather than attempting to claim exclusive hardware locks on raw physical devices (e.g. hw:0,0).
pub fn configure_alsa_environment() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("AUDIODEV").is_none() {
            // SAFETY: Modifying environment variable before spawning threads / during single-threaded startup.
            unsafe {
                std::env::set_var("AUDIODEV", "default");
            }
        }
    }
}

/// Represents the status of the underlying audio driver and hardware device lock.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub enum AudioDriverStatus {
    /// Audio driver has not been probed yet.
    Uninitialized,
    /// Audio driver lock successfully acquired and output device is ready.
    Available {
        backend: String,
        device: String,
    },
    /// Audio driver failed to acquire a device lock (e.g., EBUSY, DeviceNotAvailable,
    /// missing sound daemon, or exclusive hardware contention).
    FailedLock {
        reason: String,
        attempts: u32,
    },
    /// Audio explicitly disabled.
    Disabled,
}

impl Default for AudioDriverStatus {
    fn default() -> Self {
        // Disabled by default for now to prevent ALSA alsa::poll() device contention errors on Linux
        AudioDriverStatus::Disabled
    }
}

impl AudioDriverStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, AudioDriverStatus::Available { .. })
    }

    pub fn is_failed_lock(&self) -> bool {
        matches!(self, AudioDriverStatus::FailedLock { .. })
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, AudioDriverStatus::Disabled)
    }

    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            AudioDriverStatus::FailedLock { reason, .. } => Some(reason.as_str()),
            _ => None,
        }
    }
}

/// Audio metrics to monitor execution and verify that driver initialization is not
/// spammed on every frame.
#[derive(Resource, Debug, Clone, Default)]
pub struct AudioMetrics {
    /// Total number of driver initialization attempts (must remain 1 if startup lock fails).
    pub initialization_attempts: u32,
    /// Total number of frames elapsed during audio system execution.
    pub frames_elapsed: u64,
    /// Total sound events received from gameplay systems.
    pub sound_events_received: u32,
    /// Total sound events successfully processed for playback.
    pub sound_events_played: u32,
    /// Total sound events suppressed safely due to unavailable or locked audio driver.
    pub sound_events_suppressed: u32,
}

/// Configuration settings for workshop audio.
#[derive(Resource, Debug, Clone)]
pub struct AudioConfig {
    /// Master switch for audio. Disabled by default to prevent ALSA device polling and alsa::poll() errors.
    pub enabled: bool,
    pub master_volume: f32,
    /// Whether automatic reinitialization retries are permitted.
    /// Kept false by default to strictly avoid per-frame reinstantiation loops.
    pub allow_reinit_retry: bool,
    /// Minimum time in seconds between retry attempts when manually triggered.
    pub min_retry_interval_secs: f32,
    /// Timestamp of the last retry attempt.
    pub last_retry_time: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default for now
            master_volume: 1.0,
            allow_reinit_retry: false,
            min_retry_interval_secs: 30.0,
            last_retry_time: -30.0,
        }
    }
}

/// Distinct gameplay sound cues supported by the workshop sound system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundCueType {
    MooseHuff,
    MooseWindowSlam,
    VacuumSuction,
    PresentDelivered,
    ToolScissors,
    ToolTape,
}

/// Message to play a specific sound cue with optional volume scaling.
#[derive(Message, Clone, Debug)]
pub struct PlaySoundMessage {
    pub cue: SoundCueType,
    pub volume: f32,
}

impl PlaySoundMessage {
    pub fn new(cue: SoundCueType) -> Self {
        Self { cue, volume: 1.0 }
    }
}

/// Message to explicitly request an audio driver reinitialization attempt (subject to rate limiting).
#[derive(Message, Clone, Debug, Default)]
pub struct RetryAudioInitMessage;

/// Probes the audio host and default output device for lock acquisition using CPAL.
pub fn probe_audio_device() -> Result<(String, String), String> {
    configure_alsa_environment();

    let host = cpal::default_host();
    let backend_name = host.id().name().to_string();

    let default_device = host.default_output_device().ok_or_else(|| {
        "No default audio output device found on host. The audio daemon (PipeWire/PulseAudio) \
         may be absent, or ALSA 'default' routing is unavailable."
            .to_string()
    })?;

    let device_name = <cpal::Device as DeviceTrait>::description(&default_device)
        .map(|desc| desc.name().to_string())
        .unwrap_or_else(|_| "Default Device".to_string());

    // Verify whether the device configuration and lock can actually be acquired without EBUSY
    match default_device.default_output_config() {
        Ok(_) => Ok((backend_name, device_name)),
        Err(err) => Err(format!(
            "Failed to acquire audio device lock for '{}' on {} backend ({:?}). \
             ALSA device may be locked exclusively by another process.",
            device_name, backend_name, err
        )),
    }
}

/// One-shot startup system: attempts audio driver lock acquisition exactly once if audio is enabled.
/// If audio is disabled by default, or if lock acquisition fails, it leaves audio disabled without touching ALSA.
pub fn init_audio_driver_system(
    mut status: ResMut<AudioDriverStatus>,
    mut metrics: ResMut<AudioMetrics>,
    config: Res<AudioConfig>,
) {
    // If status has been explicitly initialized or overridden (e.g. FailedLock, Available, or Disabled), preserve it
    if *status != AudioDriverStatus::Uninitialized {
        return;
    }

    // If audio is disabled by default in config, set status to Disabled and skip ALSA probe
    if !config.enabled {
        *status = AudioDriverStatus::Disabled;
        info!("Audio is disabled by default. Skipping ALSA/CPAL driver probe to avoid alsa::poll() contention.");
        return;
    }

    metrics.initialization_attempts += 1;

    match probe_audio_device() {
        Ok((backend, device)) => {
            info!(
                "Audio driver initialized successfully (backend: {}, device: {}).",
                backend, device
            );
            *status = AudioDriverStatus::Available { backend, device };
        }
        Err(reason) => {
            warn!(
                "Audio driver lock failed: {}. Audio safely disabled; \
                 will NOT poll or reinstantiate audio driver per frame.",
                reason
            );
            *status = AudioDriverStatus::FailedLock {
                reason,
                attempts: metrics.initialization_attempts,
            };
        }
    }
}

/// Runs every frame to track elapsed frames. Critically, does NOT attempt any ALSA
/// driver re-open calls when in `FailedLock` or `Disabled` state.
pub fn audio_frame_tick_system(
    mut metrics: ResMut<AudioMetrics>,
    _status: Res<AudioDriverStatus>,
) {
    metrics.frames_elapsed += 1;
}

/// Dispatches gameplay sound events. If the audio driver is unavailable or locked,
/// events are cleanly suppressed and discarded without invoking blocking ALSA syscalls.
pub fn handle_audio_events_system(
    mut metrics: ResMut<AudioMetrics>,
    status: Res<AudioDriverStatus>,
    huff_events: Option<MessageReader<MooseHuffEvent>>,
    slam_events: Option<MessageReader<MooseWindowSlamEvent>>,
    play_sound_messages: Option<MessageReader<PlaySoundMessage>>,
) {
    let mut pending_cues = Vec::new();

    if let Some(mut reader) = huff_events {
        for _ in reader.read() {
            pending_cues.push(SoundCueType::MooseHuff);
        }
    }

    if let Some(mut reader) = slam_events {
        for _ in reader.read() {
            pending_cues.push(SoundCueType::MooseWindowSlam);
        }
    }

    if let Some(mut reader) = play_sound_messages {
        for msg in reader.read() {
            pending_cues.push(msg.cue);
        }
    }

    if pending_cues.is_empty() {
        return;
    }

    metrics.sound_events_received += pending_cues.len() as u32;

    if status.is_available() {
        // Output device is available: play cues
        for cue in pending_cues {
            debug!("Playing audio cue: {:?}", cue);
            metrics.sound_events_played += 1;
        }
    } else {
        // Audio driver lock unavailable: safely suppress playback without reinstantiating ALSA
        metrics.sound_events_suppressed += pending_cues.len() as u32;
    }
}

/// Handles explicit user-requested reinitialization. Enforces rate-limiting to prevent
/// rapid polling or per-frame reconnect storms.
pub fn handle_audio_retry_system(
    mut status: ResMut<AudioDriverStatus>,
    mut metrics: ResMut<AudioMetrics>,
    mut config: ResMut<AudioConfig>,
    time: Option<Res<Time>>,
    retry_messages: Option<MessageReader<RetryAudioInitMessage>>,
) {
    let Some(mut reader) = retry_messages else {
        return;
    };

    let has_requests = reader.read().next().is_some();
    if !has_requests {
        return;
    }

    let current_time = time.as_ref().map_or(0.0, |t| t.elapsed_secs());

    if !config.allow_reinit_retry {
        debug!("Audio reinitialization request ignored (allow_reinit_retry is disabled).");
        return;
    }

    if current_time - config.last_retry_time < config.min_retry_interval_secs {
        debug!(
            "Audio reinitialization rate-limited (minimum {}s cooldown). Skipping.",
            config.min_retry_interval_secs
        );
        return;
    }

    config.last_retry_time = current_time;
    metrics.initialization_attempts += 1;

    info!("Attempting rate-limited audio driver re-initialization (attempt {})...", metrics.initialization_attempts);

    match probe_audio_device() {
        Ok((backend, device)) => {
            info!("Audio driver re-acquired successfully (backend: {}, device: {}).", backend, device);
            *status = AudioDriverStatus::Available { backend, device };
        }
        Err(reason) => {
            warn!("Audio driver re-acquisition failed: {}. Continuing in disabled mode.", reason);
            *status = AudioDriverStatus::FailedLock {
                reason,
                attempts: metrics.initialization_attempts,
            };
        }
    }
}

/// Dedicated plugin for Santa's Workshop safe audio management.
pub struct ElfAudioPlugin;

impl Plugin for ElfAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioDriverStatus>()
            .init_resource::<AudioMetrics>()
            .init_resource::<AudioConfig>()
            .add_message::<PlaySoundMessage>()
            .add_message::<RetryAudioInitMessage>()
            .add_systems(Startup, init_audio_driver_system)
            .add_systems(
                Update,
                (
                    audio_frame_tick_system,
                    handle_audio_events_system,
                    handle_audio_retry_system,
                ),
            );
    }
}
