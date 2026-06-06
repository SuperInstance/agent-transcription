//! Take agent session events and transcribe them into a musical score.
//! Shows three transcription styles and plays back the result.

use agent_transcription::*;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     TRANSCRIPTION DEMO — Sessions Become Scores            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Simulate a multi-agent session
    let events = vec![
        SessionEvent::new(0,    "planner",  ActionType::Decision,  0.9),
        SessionEvent::new(200,  "planner",  ActionType::Message,   0.6),
        SessionEvent::new(400,  "coder",    ActionType::ToolCall,  0.8),
        SessionEvent::new(600,  "coder",    ActionType::ToolCall,  0.7),
        SessionEvent::new(900,  "reviewer", ActionType::Message,   0.5),
        SessionEvent::new(1100, "coder",    ActionType::Error,     0.3),
        SessionEvent::new(1300, "planner",  ActionType::Decision,  0.7),
        SessionEvent::new(1500, "coder",    ActionType::ToolCall,  0.9),
        SessionEvent::new(1800, "reviewer", ActionType::Message,   0.4),
        SessionEvent::new(2000, "coder",    ActionType::Completion,1.0),
        SessionEvent::new(2200, "reviewer", ActionType::Completion,0.8),
        SessionEvent::new(2400, "planner",  ActionType::Completion,0.9),
        SessionEvent::new(2600, "planner",  ActionType::Idle,      0.1),
        SessionEvent::new(2800, "coder",    ActionType::Idle,      0.1),
    ];

    println!("Session events ({} total):", events.len());
    for e in &events {
        let icon = match e.action_type {
            ActionType::Message   => "💬",
            ActionType::ToolCall  => "🔧",
            ActionType::Decision  => "🎯",
            ActionType::Idle      => "💤",
            ActionType::Error     => "❌",
            ActionType::Completion => "✅",
        };
        println!("  {} {:>5}ms  {:<10} {:?}  intensity={:.1}",
            icon, e.timestamp, e.agent_id, e.action_type, e.intensity);
    }
    println!();

    // Transcribe in all three styles
    let styles = [
        (TranscriptionStyle::Literal, "Literal", "Direct mapping: action → pitch"),
        (TranscriptionStyle::Melodic { scale_intervals: vec![0, 2, 4, 5, 7, 9, 11] },
            "Melodic (Major)", "Events form a major scale melody"),
        (TranscriptionStyle::Abstract, "Abstract", "Atmospheric textures and clusters"),
    ];

    for (style, name, desc) in &styles {
        println!("━━━ {} Style ━━━", name);
        println!("  {}", desc);
        let score = transcribe(&events, style.clone(), format!("{} Session", name));

        println!("  Notes: {} | Duration: {}ms | Voices: {}",
            score.note_count(), score.duration_ms(), score.voices().join(", "));

        // Show each note
        for note in &score.notes {
            let note_name = midi_to_name(note.pitch);
            let vol_bar: String = "▓".repeat((note.velocity as usize * 20 / 127).max(1));
            println!("    {:>5}ms  {:<10} {:>3} ({:>3}) vel={:>3} dur={:>4}ms |{}",
                note.offset_ms, note.voice, note.pitch, note_name, note.velocity, note.duration_ms, vol_bar);
        }
        println!();
    }

    // Playback simulation
    println!("━━━ Playback Simulation (Literal, 500ms ticks) ━━━");
    let score = transcribe(&events, TranscriptionStyle::Literal, "Playback");
    let mut player = SessionPlayer::new(score);
    let mut tick = 0;

    while !player.is_finished() {
        let notes = player.tick(500);
        if !notes.is_empty() {
            let pitches: Vec<String> = notes.iter()
                .map(|n| format!("{}({})", midi_to_name(n.pitch), n.voice))
                .collect();
            println!("  tick {:>2} [{:>5}-{:<5}ms]: {}",
                tick, tick * 500, (tick + 1) * 500, pitches.join(", "));
        } else {
            println!("  tick {:>2} [{:>5}-{:<5}ms]: (silence)", tick, tick * 500, (tick + 1) * 500);
        }
        tick += 1;
    }

    // Split by voice
    println!();
    println!("━━━ Per-Voice Scores ━━━");
    let score = transcribe(&events, TranscriptionStyle::Literal, "Split");
    let voices = split_by_voice(&score);
    for (name, sub_score) in &voices {
        println!("  {}: {} notes, {:.1}s duration", name, sub_score.note_count(), sub_score.duration_ms() as f64 / 1000.0);
    }
}

fn midi_to_name(midi: u8) -> String {
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = midi / 12;
    let note = (midi % 12) as usize;
    format!("{}{}", names[note], octave.saturating_sub(1))
}
