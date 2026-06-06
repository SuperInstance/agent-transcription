# agent-transcription

**Transcribing agent sessions into musical scores.**

Every AI agent session is a performance — decisions, tool calls, messages, errors — each an event with timing, intensity, and character. `agent-transcription` maps these events to musical parameters (pitch, duration, velocity, offset) and produces playable scores from session logs.

## Core Concepts

### SessionEvent

The fundamental unit. Each event captures:
- **timestamp** — when it happened (ms from session start)
- **agent_id** — which agent performed it (becomes a voice/track)
- **action_type** — what kind of action (Message, ToolCall, Decision, Idle, Error, Completion)
- **intensity** — energy level [0.0, 1.0], maps to velocity and duration

### TranscriptionStyle

How events become notes:

| Style | Mapping |
|-------|---------|
| **Literal** | action_type → fixed pitch, intensity → velocity, timestamp → offset |
| **Melodic** | pitches chosen from a musical scale so events form melodies |
| **Abstract** | events mapped to atmospheric clusters and textures |

### SessionScore

A polyphonic score with:
- Multiple voices (one per agent)
- Tempo marking
- Note-level detail (pitch, duration, velocity, offset)

### SessionPlayer

Time-based playback engine. Call `tick(dt)` to advance and receive notes that start within that window. Supports reset and completion detection.

## Usage

```rust
use agent_transcription::*;

let events = vec![
    SessionEvent::new(0, "agent-a", ActionType::Message, 0.5),
    SessionEvent::new(500, "agent-a", ActionType::ToolCall, 0.8),
    SessionEvent::new(1000, "agent-b", ActionType::Decision, 0.6),
];

// Transcribe with a minor pentatonic scale
let score = transcribe(&events,
    TranscriptionStyle::Melodic { scale_intervals: vec![0, 3, 5, 7, 10] },
    "session-2024-01-15"
);

// Split into per-agent scores
let voices = split_by_voice(&score);
for (agent, sub_score) in &voices {
    println!("{}: {} notes", agent, sub_score.note_count());
}

// Play back
let mut player = SessionPlayer::new(score);
while !player.is_finished() {
    let notes = player.tick(100); // 100ms steps
    for note in &notes {
        println!("t={}ms voice={} pitch={} vel={}",
            note.offset_ms, note.voice, note.pitch, note.velocity);
    }
}
```

## Multi-Agent Sessions

When multiple agents operate in a session, each becomes a separate voice in the score. `transcribe_multi` handles this naturally — same events, same function, polyphonic output. `split_by_voice` extracts individual parts.

## Design Philosophy

Agent sessions have rhythm. A busy agent fires rapid tool calls; a thoughtful one spaces out decisions. Errors spike in clusters. Completions land like cadences. This crate makes that rhythm audible.

The three transcription styles reflect different listening modes:
- **Literal** — forensic analysis, each action type is distinct
- **Melodic** — aesthetic, the session becomes a tune
- **Abstract** — atmospheric, good for ambient representation

## Testing

14 tests covering event creation, all three mapping styles, score generation, style differentiation, multi-agent transcription, playback, edge cases (empty sessions, clamped intensities), and complete action type coverage.

```bash
cargo test
```

## License

MIT
