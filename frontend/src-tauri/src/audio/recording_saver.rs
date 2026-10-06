use std::sync::{Arc, Mutex};
use anyhow::Result;
use log::{info, warn, error};
use tauri::{AppHandle, Runtime, Emitter};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

use super::audio_processing::create_meeting_folder;

/// Structured transcript segment for JSON export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: String,
    pub text: String,
    pub audio_start_time: f64, // Seconds from recording start
    pub audio_end_time: f64,   // Seconds from recording start
    pub duration: f64,          // Segment duration in seconds
    pub display_time: String,   // Formatted time for display like "[02:15]"
    pub confidence: f32,
    pub sequence_id: u64,
}

/// Meeting metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingMetadata {
    pub version: String,
    pub meeting_id: Option<String>,
    pub meeting_name: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub duration_seconds: Option<f64>,
    pub devices: DeviceInfo,
    pub audio_file: String,
    pub transcript_file: String,
    pub sample_rate: u32,
    pub status: String,  // "recording", "completed", "error"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub microphone: Option<String>,
    pub system_audio: Option<String>,
}

/// Persists transcript text and meeting metadata only. Never receives audio samples.
pub struct RecordingSaver {
    meeting_folder: Option<PathBuf>,
    meeting_name: Option<String>,
    metadata: Option<MeetingMetadata>,
    transcript_segments: Arc<Mutex<Vec<TranscriptSegment>>>,
}

impl RecordingSaver {
    pub fn new() -> Self {
        Self {
            meeting_folder: None,
            meeting_name: None,
            metadata: None,
            transcript_segments: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Set the meeting name for this recording session
    pub fn set_meeting_name(&mut self, name: Option<String>) {
        self.meeting_name = name;
    }

    /// Set device information in metadata
    pub fn set_device_info(&mut self, mic_name: Option<String>, sys_name: Option<String>) {
        if let Some(ref mut metadata) = self.metadata {
            metadata.devices.microphone = mic_name;
            metadata.devices.system_audio = sys_name;

            // Write updated metadata to disk if folder exists
            if let Some(folder) = &self.meeting_folder {
                let metadata_clone = metadata.clone();
                if let Err(e) = self.write_metadata(folder, &metadata_clone) {
                    warn!("Failed to update metadata with device info: {}", e);
                }
            }
        }
    }

    /// Add or update a structured transcript segment (upserts based on sequence_id)
    /// Also saves incrementally to disk
    pub fn add_transcript_segment(&self, segment: TranscriptSegment) {
        if let Ok(mut segments) = self.transcript_segments.lock() {
            // Check if segment with same sequence_id exists (update it)
            if let Some(existing) = segments.iter_mut().find(|s| s.sequence_id == segment.sequence_id) {
                *existing = segment.clone();
                info!("Updated transcript segment {} (seq: {}) - total segments: {}",
                      segment.id, segment.sequence_id, segments.len());
            } else {
                // New segment, add it
                segments.push(segment.clone());
                info!("Added new transcript segment {} (seq: {}) - total segments: {}",
                      segment.id, segment.sequence_id, segments.len());
            }
        } else {
            error!("Failed to lock transcript segments for adding segment {}", segment.id);
        }

        // NEW: Save incrementally to disk
        if let Some(folder) = &self.meeting_folder {
            if let Err(e) = self.write_transcripts_json(folder) {
                warn!("Failed to write incremental transcript update: {}", e);
            }
        }
    }

    /// Legacy method for backward compatibility - converts text to basic segment
    pub fn add_transcript_chunk(&self, text: String) {
        let segment = TranscriptSegment {
            id: format!("seg_{}", chrono::Utc::now().timestamp_millis()),
            text,
            audio_start_time: 0.0,
            audio_end_time: 0.0,
            duration: 0.0,
            display_time: "[00:00]".to_string(),
            confidence: 1.0,
            sequence_id: 0,
        };
        self.add_transcript_segment(segment);
    }

    /// Start transcript persistence. Audio stays in the transcription pipeline.
    pub fn start_session(&mut self) -> Result<()> {
        let name = self.meeting_name.clone().unwrap_or_else(|| "Meeting".to_string());
        self.initialize_meeting_folder(
            &super::recording_preferences::get_default_recordings_folder(),
            &name,
        )
    }

    fn initialize_meeting_folder(&mut self, base_folder: &PathBuf, meeting_name: &str) -> Result<()> {
        let meeting_folder = create_meeting_folder(base_folder, meeting_name)?;

        // Create initial metadata
        let metadata = MeetingMetadata {
            version: "1.0".to_string(),
            meeting_id: None,  // Will be set by backend
            meeting_name: Some(meeting_name.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            duration_seconds: None,
            devices: DeviceInfo {
                microphone: None,  // Could be enhanced to store actual device names
                system_audio: None,
            },
            audio_file: String::new(),
            transcript_file: "transcripts.json".to_string(),
            sample_rate: 48000,
            status: "recording".to_string(),
        };

        // Write initial metadata.json
        self.write_metadata(&meeting_folder, &metadata)?;

        self.meeting_folder = Some(meeting_folder);
        self.metadata = Some(metadata);

        Ok(())
    }

    /// Write metadata.json to disk (atomic write with temp file)
    fn write_metadata(&self, folder: &PathBuf, metadata: &MeetingMetadata) -> Result<()> {
        let metadata_path = folder.join("metadata.json");
        let temp_path = folder.join(".metadata.json.tmp");

        let json_string = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&temp_path, json_string)?;
        std::fs::rename(&temp_path, &metadata_path)?;  // Atomic

        Ok(())
    }

    /// Write transcripts.json to disk (atomic write with temp file and validation)
    fn write_transcripts_json(&self, folder: &PathBuf) -> Result<()> {
        // Clone segments to avoid holding lock during I/O
        let segments_clone = if let Ok(segments) = self.transcript_segments.lock() {
            segments.clone()
        } else {
            error!("Failed to lock transcript segments for writing");
            return Err(anyhow::anyhow!("Failed to lock transcript segments"));
        };

        info!("Writing {} transcript segments to JSON", segments_clone.len());

        let transcript_path = folder.join("transcripts.json");
        let temp_path = folder.join(".transcripts.json.tmp");

        // Create JSON structure
        let json = serde_json::json!({
            "version": "1.0",
            "segments": segments_clone,
            "last_updated": chrono::Utc::now().to_rfc3339(),
            "total_segments": segments_clone.len()
        });

        // Serialize to pretty JSON string
        let json_string = serde_json::to_string_pretty(&json)
            .map_err(|e| {
                error!("Failed to serialize transcripts to JSON: {}", e);
                anyhow::anyhow!("JSON serialization failed: {}", e)
            })?;

        // Write to temp file with error handling
        std::fs::write(&temp_path, &json_string)
            .map_err(|e| {
                error!("Failed to write transcript temp file to {}: {}", temp_path.display(), e);
                anyhow::anyhow!("Failed to write temp file: {}", e)
            })?;

        // Verify temp file was written correctly
        if !temp_path.exists() {
            error!("Temp transcript file does not exist after write: {}", temp_path.display());
            return Err(anyhow::anyhow!("Temp file verification failed"));
        }

        // Atomic rename
        std::fs::rename(&temp_path, &transcript_path)
            .map_err(|e| {
                error!("Failed to rename transcript file from {} to {}: {}",
                       temp_path.display(), transcript_path.display(), e);
                anyhow::anyhow!("Failed to rename transcript file: {}", e)
            })?;

        info!("✅ Successfully wrote transcripts.json with {} segments", segments_clone.len());
        Ok(())
    }

    pub fn get_stats(&self) -> (usize, u32) {
        (0, 48000)
    }

    /// Finish the text files even for silent sessions or interrupted transcription.
    fn finish_transcripts(&mut self, recording_duration: Option<f64>) -> Result<(), String> {
        // Save final transcripts.json with validation
        if let Some(folder) = &self.meeting_folder {
            if let Err(e) = self.write_transcripts_json(folder) {
                error!("❌ Failed to write final transcripts: {}", e);
                return Err(format!("Failed to save transcripts: {}", e));
            }

            // Verify transcripts were written correctly
            let transcript_path = folder.join("transcripts.json");
            if !transcript_path.exists() {
                error!("❌ Transcript file was not created at: {}", transcript_path.display());
                return Err("Transcript file verification failed".to_string());
            }
            info!("✅ Transcripts saved and verified at: {}", transcript_path.display());
        }

        // Update metadata to completed status with actual recording duration
        if let (Some(folder), Some(mut metadata)) = (&self.meeting_folder, self.metadata.clone()) {
            metadata.status = "completed".to_string();
            metadata.completed_at = Some(chrono::Utc::now().to_rfc3339());

            // Use actual recording duration from RecordingState (more accurate than transcript segments)
            // Falls back to last transcript segment if duration not provided
            metadata.duration_seconds = recording_duration.or_else(|| {
                if let Ok(segments) = self.transcript_segments.lock() {
                    segments.last().map(|seg| seg.audio_end_time)
                } else {
                    None
                }
            });

            if let Err(e) = self.write_metadata(folder, &metadata) {
                error!("❌ Failed to update metadata to completed: {}", e);
                return Err(format!("Failed to update metadata: {}", e));
            }

            info!("✅ Metadata updated with duration: {:?}s", metadata.duration_seconds);
        }

        Ok(())
    }

    pub async fn stop_and_save<R: Runtime>(
        &mut self,
        app: &AppHandle<R>,
        recording_duration: Option<f64>,
    ) -> Result<Option<String>, String> {
        self.finish_transcripts(recording_duration)?;
        let transcript_path = self.meeting_folder.as_ref()
            .map(|folder| folder.join("transcripts.json").to_string_lossy().to_string());
        let save_event = serde_json::json!({
            "audio_file": null,
            "transcript_file": transcript_path,
            "meeting_name": self.meeting_name,
            "meeting_folder": self.meeting_folder.as_ref()
                .map(|folder| folder.to_string_lossy().to_string())
        });
        if let Err(e) = app.emit("recording-saved", &save_event) {
            warn!("Failed to emit recording-saved event: {}", e);
        }
        // Keep the in-memory transcript available for reload sync until the manager is dropped.
        Ok(transcript_path)
    }

    /// Get the meeting folder path (for passing to backend)
    pub fn get_meeting_folder(&self) -> Option<&PathBuf> {
        self.meeting_folder.as_ref()
    }

    /// Get accumulated transcript segments (for reload sync)
    pub fn get_transcript_segments(&self) -> Vec<TranscriptSegment> {
        if let Ok(segments) = self.transcript_segments.lock() {
            segments.clone()
        } else {
            Vec::new()
        }
    }

    /// Get meeting name (for reload sync)
    pub fn get_meeting_name(&self) -> Option<String> {
        self.meeting_name.clone()
    }
}

impl Default for RecordingSaver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    fn files(folder: &PathBuf) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(folder).unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn privacy_session_saves_only_text_and_finishes_metadata() {
        let root = tempfile::tempdir().unwrap();
        let mut saver = RecordingSaver::new();
        saver.initialize_meeting_folder(&root.path().to_path_buf(), "Example").unwrap();
        let folder = saver.get_meeting_folder().unwrap().clone();
        assert_eq!(files(&folder), ["metadata.json"]);
        let mut segment = TranscriptSegment {
            id: "segment-1".into(), text: "Draft text".into(), audio_start_time: 0.0,
            audio_end_time: 2.5, duration: 2.5, display_time: "[00:00]".into(),
            confidence: 1.0, sequence_id: 1,
        };
        saver.add_transcript_segment(segment.clone());
        segment.text = "Final text".into();
        saver.add_transcript_segment(segment);
        // Text is recoverable before a normal stop, without any audio checkpoint.
        let before: serde_json::Value = serde_json::from_slice(
            &std::fs::read(folder.join("transcripts.json")).unwrap()).unwrap();
        assert_eq!(before["segments"].as_array().unwrap().len(), 1);
        assert_eq!(before["segments"][0]["text"], "Final text");
        saver.finish_transcripts(Some(4.0)).unwrap();
        let metadata: MeetingMetadata = serde_json::from_slice(
            &std::fs::read(folder.join("metadata.json")).unwrap()).unwrap();
        assert_eq!(metadata.audio_file, "");
        assert_eq!(metadata.status, "completed");
        assert!(metadata.completed_at.is_some());
        assert_eq!(metadata.duration_seconds, Some(4.0));
        assert_eq!(files(&folder), ["metadata.json", "transcripts.json"]);
        assert_eq!(saver.get_transcript_segments().len(), 1);
    }

    #[test]
    fn privacy_sessions_do_not_reuse_older_audio_folders() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().to_path_buf();
        let old_folder = create_meeting_folder(&base, "Repeated title").unwrap();
        std::fs::write(old_folder.join("audio.mp4"), b"existing user file").unwrap();
        let mut saver = RecordingSaver::new();
        saver.initialize_meeting_folder(&base, "Repeated title").unwrap();
        saver.finish_transcripts(Some(0.0)).unwrap();
        let new_folder = saver.get_meeting_folder().unwrap();
        assert_ne!(new_folder, &old_folder);
        assert_eq!(files(new_folder), ["metadata.json", "transcripts.json"]);
        assert_eq!(std::fs::read(old_folder.join("audio.mp4")).unwrap(), b"existing user file");
    }

    #[test]
    fn privacy_silent_session_has_no_audio_or_checkpoints() {
        let root = tempfile::tempdir().unwrap();
        let mut saver = RecordingSaver::new();
        saver.initialize_meeting_folder(&root.path().to_path_buf(), "Silence").unwrap();
        saver.finish_transcripts(Some(10.0)).unwrap();
        let folder = saver.get_meeting_folder().unwrap();
        assert_eq!(files(folder), ["metadata.json", "transcripts.json"]);
        let transcript: serde_json::Value = serde_json::from_slice(
            &std::fs::read(folder.join("transcripts.json")).unwrap()).unwrap();
        assert_eq!(transcript["total_segments"], 0);
    }
}
