//! # agent-transcription
//!
//! Transcribe agent sessions into playable musical scores. Each agent event —
//! a decision, a tool call, a message — becomes a note. Sessions become
//! compositions.

use std::collections::HashMap;

/// The type of action an agent performed during a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionType {
    /// Agent sent a message to a user or channel.
    Message,
    /// Agent invoked a tool (search, exec, read, etc.).
    ToolCall,
    /// Agent made a decision or chose a path.
    Decision,
    /// Agent was idle or waiting.
    Idle,
    /// Agent produced an error or failure.
    Error,
    /// Agent completed a sub-task.
    Completion,
}

/// A single event from an agent session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionEvent {
    /// Milliseconds since session start.
    pub timestamp: u64,
    /// Which agent produced this event.
    pub agent_id: String,
    /// What kind of action occurred.
    pub action_type: ActionType,
    /// Intensity 0.0–1.0 (e.g. urgency, energy level).
    pub intensity: f64,
}

impl SessionEvent {
    pub fn new(timestamp: u64, agent_id: impl Into<String>, action_type: ActionType, intensity: f64) -> Self {
        Self {
            timestamp,
            agent_id: agent_id.into(),
            action_type,
            intensity: intensity.clamp(0.0, 1.0),
        }
    }
}

/// A single musical note in a score.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// MIDI pitch 0–127.
    pub pitch: u8,
    /// Duration in milliseconds.
    pub duration_ms: u32,
    /// Velocity 0–127.
    pub velocity: u8,
    /// Offset in milliseconds from score start.
    pub offset_ms: u32,
    /// Which voice/track this note belongs to (one per agent).
    pub voice: String,
}

/// A musical score produced from one or more agent sessions.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionScore {
    pub notes: Vec<Note>,
    pub tempo_bpm: f64,
    pub title: String,
}

impl SessionScore {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            notes: Vec::new(),
            tempo_bpm: 120.0,
            title: title.into(),
        }
    }

    pub fn duration_ms(&self) -> u32 {
        self.notes.iter().map(|n| n.offset_ms + n.duration_ms).max().unwrap_or(0)
    }

    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    pub fn voices(&self) -> Vec<String> {
        let mut v: Vec<String> = self.notes.iter().map(|n| n.voice.clone()).collect();
        v.sort();
        v.dedup();
        v
    }
}

/// Style of transcription — how events map to musical parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum TranscriptionStyle {
    /// Literal: action_type → fixed pitch, intensity → velocity, timestamp → offset.
    Literal,
    /// Melodic: pitches are chosen from a scale so consecutive events form a melody.
    Melodic { scale_intervals: Vec<u8> },
    /// Abstract: events are mapped to atmospheric textures, clusters, and dynamics.
    Abstract,
}

impl Default for TranscriptionStyle {
    fn default() -> Self {
        Self::Literal
    }
}

/// Maps events to musical notes.
pub trait EventToNote {
    fn map(&self, event: &SessionEvent, voice: &str) -> Note;
}

/// Default mapper that respects the transcription style.
#[derive(Debug, Clone)]
pub struct DefaultMapper {
    pub style: TranscriptionStyle,
    /// Base MIDI pitch (C4 = 60).
    pub base_pitch: u8,
    /// Minimum note duration in ms.
    pub min_duration_ms: u32,
    /// Maximum note duration in ms.
    pub max_duration_ms: u32,
}

impl DefaultMapper {
    pub fn new(style: TranscriptionStyle) -> Self {
        Self {
            style,
            base_pitch: 60,
            min_duration_ms: 100,
            max_duration_ms: 2000,
        }
    }

    fn action_pitch_literal(&self, action: &ActionType) -> u8 {
        match action {
            ActionType::Message => self.base_pitch,
            ActionType::ToolCall => self.base_pitch + 4,
            ActionType::Decision => self.base_pitch + 7,
            ActionType::Idle => self.base_pitch - 12,
            ActionType::Error => self.base_pitch + 11,
            ActionType::Completion => self.base_pitch + 12,
        }
    }

    fn action_pitch_melodic(&self, _action: &ActionType, scale: &[u8], idx: usize) -> u8 {
        if scale.is_empty() {
            return self.base_pitch;
        }
        let interval = scale[idx % scale.len()];
        self.base_pitch + interval
    }

    fn action_pitch_abstract(&self, event: &SessionEvent) -> u8 {
        // Map intensity + action_type hash into a wider range
        let base = match &event.action_type {
            ActionType::Message => 48,
            ActionType::ToolCall => 55,
            ActionType::Decision => 62,
            ActionType::Idle => 36,
            ActionType::Error => 70,
            ActionType::Completion => 67,
        };
        let spread = (event.intensity * 24.0) as u8;
        base + spread % 25
    }
}

impl EventToNote for DefaultMapper {
    fn map(&self, event: &SessionEvent, voice: &str) -> Note {
        let pitch = match &self.style {
            TranscriptionStyle::Literal => self.action_pitch_literal(&event.action_type),
            TranscriptionStyle::Melodic { scale_intervals } => {
                let idx = (event.timestamp / 500) as usize;
                self.action_pitch_melodic(&event.action_type, scale_intervals, idx)
            }
            TranscriptionStyle::Abstract => self.action_pitch_abstract(event),
        };

        let velocity = (event.intensity * 127.0).min(127.0) as u8;
        let duration_range = self.max_duration_ms - self.min_duration_ms;
        let duration = self.min_duration_ms + (event.intensity * duration_range as f64) as u32;

        Note {
            pitch: pitch.min(127),
            duration_ms: duration,
            velocity: if velocity == 0 { 1 } else { velocity },
            offset_ms: event.timestamp as u32,
            voice: voice.to_string(),
        }
    }
}

/// Replays a session score, yielding notes at the correct time.
pub struct SessionPlayer {
    score: SessionScore,
    position_ms: u32,
}

impl SessionPlayer {
    pub fn new(score: SessionScore) -> Self {
        Self { score, position_ms: 0 }
    }

    /// Advance by `dt` milliseconds and return notes that start in this window.
    pub fn tick(&mut self, dt_ms: u32) -> Vec<Note> {
        let start = self.position_ms;
        let end = start + dt_ms;
        let notes: Vec<Note> = self.score.notes.iter()
            .filter(|n| n.offset_ms >= start && n.offset_ms < end)
            .cloned()
            .collect();
        self.position_ms = end;
        notes
    }

    /// Whether playback has finished.
    pub fn is_finished(&self) -> bool {
        self.position_ms >= self.score.duration_ms()
    }

    pub fn position_ms(&self) -> u32 {
        self.position_ms
    }

    pub fn reset(&mut self) {
        self.position_ms = 0;
    }
}

/// Transcribes a full session (list of events) into a score.
pub fn transcribe(
    events: &[SessionEvent],
    style: TranscriptionStyle,
    title: impl Into<String>,
) -> SessionScore {
    let mapper = DefaultMapper::new(style);
    let mut score = SessionScore::new(title);

    for event in events {
        let note = mapper.map(event, &event.agent_id);
        score.notes.push(note);
    }

    score.notes.sort_by_key(|n| n.offset_ms);
    score
}

/// Transcribe events from multiple agents into a single polyphonic score.
pub fn transcribe_multi(
    events: &[SessionEvent],
    style: TranscriptionStyle,
    title: impl Into<String>,
) -> SessionScore {
    transcribe(events, style, title)
}

/// Extract per-agent sub-scores from a multi-agent score.
pub fn split_by_voice(score: &SessionScore) -> HashMap<String, SessionScore> {
    let mut result: HashMap<String, SessionScore> = HashMap::new();
    for note in &score.notes {
        let sub = result.entry(note.voice.clone())
            .or_insert_with(|| SessionScore::new(format!("voice-{}", note.voice)));
        sub.notes.push(note.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_events() -> Vec<SessionEvent> {
        vec![
            SessionEvent::new(0, "agent-a", ActionType::Message, 0.5),
            SessionEvent::new(500, "agent-a", ActionType::ToolCall, 0.8),
            SessionEvent::new(1000, "agent-b", ActionType::Decision, 0.6),
            SessionEvent::new(1500, "agent-a", ActionType::Completion, 1.0),
            SessionEvent::new(2000, "agent-b", ActionType::Error, 0.3),
        ]
    }

    #[test]
    fn test_event_creation() {
        let e = SessionEvent::new(100, "test", ActionType::Message, 1.5);
        assert_eq!(e.timestamp, 100);
        assert_eq!(e.agent_id, "test");
        assert_eq!(e.action_type, ActionType::Message);
        assert!((e.intensity - 1.0).abs() < f64::EPSILON); // clamped
    }

    #[test]
    fn test_literal_mapping() {
        let mapper = DefaultMapper::new(TranscriptionStyle::Literal);
        let e = SessionEvent::new(0, "a", ActionType::Message, 0.5);
        let note = mapper.map(&e, "a");
        assert_eq!(note.pitch, 60); // base_pitch for Message
        assert_eq!(note.velocity, 63); // ~0.5 * 127
        assert_eq!(note.voice, "a");
    }

    #[test]
    fn test_melodic_mapping() {
        let mapper = DefaultMapper::new(TranscriptionStyle::Melodic {
            scale_intervals: vec![0, 2, 4, 5, 7, 9, 11], // major scale
        });
        let e = SessionEvent::new(0, "a", ActionType::Message, 0.5);
        let note = mapper.map(&e, "a");
        // idx = 0/500 = 0 → interval[0] = 0 → pitch = 60
        assert_eq!(note.pitch, 60);

        let e2 = SessionEvent::new(1000, "a", ActionType::Message, 0.5);
        let note2 = mapper.map(&e2, "a");
        // idx = 1000/500 = 2 → interval[2] = 4 → pitch = 64
        assert_eq!(note2.pitch, 64);
    }

    #[test]
    fn test_abstract_mapping() {
        let mapper = DefaultMapper::new(TranscriptionStyle::Abstract);
        let e = SessionEvent::new(0, "a", ActionType::Message, 0.5);
        let note = mapper.map(&e, "a");
        assert!(note.pitch >= 48); // base for Message is 48
        assert!(note.pitch <= 127);
    }

    #[test]
    fn test_score_generation() {
        let events = sample_events();
        let score = transcribe(&events, TranscriptionStyle::Literal, "test-session");
        assert_eq!(score.note_count(), 5);
        assert_eq!(score.title, "test-session");
        // Notes should be sorted by offset
        for i in 1..score.notes.len() {
            assert!(score.notes[i].offset_ms >= score.notes[i - 1].offset_ms);
        }
    }

    #[test]
    fn test_style_differences() {
        let events = sample_events();
        let literal = transcribe(&events, TranscriptionStyle::Literal, "literal");
        let melodic = transcribe(&events, TranscriptionStyle::Melodic {
            scale_intervals: vec![0, 3, 5, 7, 10], // minor pentatonic
        }, "melodic");
        let abstract_ = transcribe(&events, TranscriptionStyle::Abstract, "abstract");

        // All three should produce same number of notes but different pitches
        assert_eq!(literal.note_count(), melodic.note_count());
        assert_eq!(literal.note_count(), abstract_.note_count());

        let pitches_eq = literal.notes.iter().zip(melodic.notes.iter())
            .all(|(a, b)| a.pitch == b.pitch);
        assert!(!pitches_eq, "Literal and melodic should produce different pitches");
    }

    #[test]
    fn test_multi_agent_transcription() {
        let events = sample_events();
        let score = transcribe_multi(&events, TranscriptionStyle::Literal, "multi");
        assert_eq!(score.voices(), vec!["agent-a", "agent-b"]);

        let split = split_by_voice(&score);
        assert_eq!(split.len(), 2);
        assert_eq!(split["agent-a"].note_count(), 3);
        assert_eq!(split["agent-b"].note_count(), 2);
    }

    #[test]
    fn test_player_tick() {
        let events = sample_events();
        let score = transcribe(&events, TranscriptionStyle::Literal, "playback");
        let mut player = SessionPlayer::new(score);

        // Tick covers [position, position+dt)
        let t0 = player.tick(501); // [0, 501) → captures 0ms and 500ms
        assert_eq!(t0.len(), 2);
        assert!(!player.is_finished());

        let t1 = player.tick(1000); // [501, 1501) → captures 1000ms and 1500ms
        assert_eq!(t1.len(), 2);

        let t2 = player.tick(600); // [1501, 2101) → captures 2000ms
        assert_eq!(t2.len(), 1);
    }

    #[test]
    fn test_player_finished_and_reset() {
        let events = vec![SessionEvent::new(0, "a", ActionType::Message, 0.5)];
        let score = transcribe(&events, TranscriptionStyle::Literal, "short");
        let mut player = SessionPlayer::new(score);

        // Note duration ≈ 1050ms, so we need to tick past that
        player.tick(2000);
        assert!(player.is_finished());

        player.reset();
        assert_eq!(player.position_ms(), 0);
        assert!(!player.is_finished());
    }

    #[test]
    fn test_score_duration() {
        let events = sample_events();
        let score = transcribe(&events, TranscriptionStyle::Literal, "dur");
        // Last event at 2000ms with some duration
        assert!(score.duration_ms() > 2000);
    }

    #[test]
    fn test_empty_session() {
        let score = transcribe(&[], TranscriptionStyle::Literal, "empty");
        assert_eq!(score.note_count(), 0);
        assert_eq!(score.duration_ms(), 0);
        assert!(score.voices().is_empty());
    }

    #[test]
    fn test_player_empty_score() {
        let score = SessionScore::new("empty");
        let mut player = SessionPlayer::new(score);
        let notes = player.tick(1000);
        assert!(notes.is_empty());
        assert!(player.is_finished());
    }

    #[test]
    fn test_intensity_clamping() {
        let e = SessionEvent::new(0, "a", ActionType::Message, -5.0);
        assert!((e.intensity - 0.0).abs() < f64::EPSILON);

        let e2 = SessionEvent::new(0, "a", ActionType::Message, 10.0);
        assert!((e2.intensity - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_all_action_types_mapped() {
        let mapper = DefaultMapper::new(TranscriptionStyle::Literal);
        let actions = [
            ActionType::Message, ActionType::ToolCall, ActionType::Decision,
            ActionType::Idle, ActionType::Error, ActionType::Completion,
        ];
        for action in &actions {
            let e = SessionEvent::new(0, "a", action.clone(), 0.5);
            let note = mapper.map(&e, "a");
            assert!(note.pitch <= 127);
            assert!(note.velocity > 0);
        }
    }
}
