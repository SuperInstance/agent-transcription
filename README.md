# agent-transcription

**Transcribing AI agent sessions into playable musical scores — every decision, tool call, and error becomes a note.**

Agent sessions have inherent rhythm: rapid tool calls form staccato bursts, long deliberations are sustained tones, errors spike as dissonance, and completions resolve like cadences. `agent-transcription` maps session events to MIDI-compatible musical parameters (pitch, velocity, duration, offset) using one of three compositional strategies, producing polyphonic scores where each agent is a separate voice.

## Why It Matters

Observability dashboards show *what* happened. Audio transcription shows *how it felt*. A session that "sounds chaotic" has rapid context-switching and error clusters; a session that "sounds flowing" has well-spaced decisions and smooth completions. This makes it valuable for:

- **Session quality assessment** — atonal or chaotic scores signal confused agent behavior
- **Multi-agent choreography** — polyphonic scores reveal whether agents are harmonizing or stepping on each other
- **Accessibility** — audio representation of system behavior for visually impaired operators
- **Aesthetic debugging** — a new modality for identifying pathological patterns

The mapping from discrete events to continuous musical parameters is a **sonification** problem, a well-studied area in auditory display science.

## How It Works

### Event-to-Note Mapping

Each `SessionEvent` has four parameters: timestamp (ms), agent_id, action_type, and intensity ∈ [0, 1]. These map to musical dimensions:

| Event Field | Musical Parameter | Mapping |
|-------------|-------------------|---------|
| timestamp | offset_ms | Direct: event time = note onset |
| intensity | velocity | Linear: v = ⌊127 · I⌉ |
| intensity | duration | Linear interpolation: d = d_min + I · (d_max - d_min) |
| agent_id | voice | One voice per agent (polyphonic) |
| action_type | pitch | Strategy-dependent (see below) |

### Pitch Assignment Strategies

**Literal** — Each action type maps to a fixed interval above base pitch (default C4 = MIDI 60):

| Action | Interval | Example |
|--------|----------|---------|
| Idle | -12 (octave below) | 48 |
| Message | +0 (unison) | 60 |
| ToolCall | +4 (major third) | 64 |
| Decision | +7 (perfect fifth) | 67 |
| Error | +11 (major seventh) | 71 |
| Completion | +12 (octave) | 72 |

This forms an ascending triadic structure (0, 4, 7) for normal actions, with dissonant/error tones outside.

**Melodic** — Pitches are drawn from a scale (e.g., major [0, 2, 4, 5, 7, 9, 11]). The scale index advances by one position every 500ms (`idx = ⌊timestamp / 500⌋ mod |scale|`), creating a melody that reflects temporal progression. This uses **diatonic modulation** — the same technique used in algorithmic composition systems.

**Abstract** — Pitch = base_action + ⌊24 · intensity⌉ mod 25, mapping intensity into a 2-octave spread. High-intensity events scatter across a wide range, creating textural density.

### Playback Engine

`SessionPlayer` advances in fixed time steps (tick-based scheduling). At each tick of Δt ms, it emits all notes whose offset falls in [position, position + Δt). This is **causal playback** — notes are only emitted after their scheduled time arrives, enabling real-time streaming.

### Complexity

| Operation | Time | Space |
|-----------|------|-------|
| Transcription (n events) | O(n) | O(n) |
| Sort by offset | O(n log n) | O(log n) |
| Player tick | O(k) where k = notes in window | O(1) |
| Split by voice | O(n) | O(n) |

## Quick Start

```rust
use agent_transcription::*;

let events = vec![
    SessionEvent::new(0, "agent-a", ActionType::Message, 0.5),
    SessionEvent::new(500, "agent-a", ActionType::ToolCall, 0.8),
    SessionEvent::new(1000, "agent-b", ActionType::Decision, 0.6),
    SessionEvent::new(1500, "agent-a", ActionType::Completion, 1.0),
];

// Transcribe with minor pentatonic scale
let score = transcribe(&events,
    TranscriptionStyle::Melodic { scale_intervals: vec![0, 3, 5, 7, 10] },
    "session-2024-01-15");

println!("{} notes, {} voices, {}ms duration",
    score.note_count(), score.voices().len(), score.duration_ms());

// Split into per-agent scores
let voices = split_by_voice(&score);

// Stream playback
let mut player = SessionPlayer::new(score);
while !player.is_finished() {
    for note in player.tick(100) {
        println!("t={}ms voice={} pitch={} vel={}",
            note.offset_ms, note.voice, note.pitch, note.velocity);
    }
}
```

## API

- **`SessionEvent`** — (timestamp, agent_id, action_type, intensity); intensity clamped to [0, 1]
- **`ActionType`** — Message, ToolCall, Decision, Idle, Error, Completion
- **`Note`** — (pitch, duration_ms, velocity, offset_ms, voice) — MIDI-compatible
- **`SessionScore`** — Vec\<Note\> + tempo + title; `duration_ms()`, `voices()`, `note_count()`
- **`TranscriptionStyle`** — Literal, Melodic { scale_intervals }, Abstract
- **`DefaultMapper`** — Configurable mapper with base_pitch, min/max duration
- **`SessionPlayer`** — Tick-based playback; `tick(dt)` → Vec\<Note\>, `is_finished()`, `reset()`
- **`transcribe()`** / **`transcribe_multi()`** — One-shot transcription functions
- **`split_by_voice()`** — Extract per-agent sub-scores

## Architecture Notes

The transcription pipeline embodies the γ+η=C principle: γ (generative diversity) is expressed through voice count and pitch variety, while η (evaluative depth) is encoded in the intensity-to-duration mapping. A session with high γ but low η produces many notes of uniform length — mechanically busy but expressively flat. The C (total complexity) is bounded by the score's information entropy.

## References

1. Kramer, G. (1994). *Auditory Display: Sonification, Audification, and Auditory Interfaces*. — foundational text on sonification.
2. Hermann, T. (2008). "Taxonomy and Definitions for Sonification and Auditory Display." *ICAD 2008*.
3. Boulanger, R. & Lazzarini, V. (2010). *The Audio Programming Book*. MIT Press. — MIDI and Csound integration.
4. Roads, C. (1996). *The Computer Music Tutorial*. MIT Press. — algorithmic composition and pitch mapping.

## License

MIT
