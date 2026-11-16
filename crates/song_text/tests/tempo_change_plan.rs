//! Tempo between sections (Musical Time and Clock, section 8): the plan says
//! where the written tempo changes, so the player can make an anchor there.
//! Checked from text as the player would send it.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::song_plan::{SongPlan, TempoChange, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;

const SONG: &str = include_str!("../../../songs_canonical/tempo_change/tempo_change.song");
const STUDIO: &str = include_str!("../../../songs_canonical/tempo_change/tempo_change.studio");

fn plan() -> SongPlan {
    let song: Song = parse_song("tempo_change.song", SONG).unwrap();
    let studio = parse_studio("tempo_change.studio", STUDIO).unwrap();
    plan_song(&song, &studio).unwrap()
}

#[test]
fn the_song_starts_at_the_first_sections_tempo() {
    assert_eq!(plan().tempo.milli_quarters_per_minute(), 100_000);
}

#[test]
fn the_tempo_changes_where_a_section_changes_the_beats_per_minute_or_the_beat() {
    let changes: Vec<(u64, u64)> = plan()
        .tempo_changes
        .iter()
        .map(|change: &TempoChange| {
            (
                change.position.ticks(),
                change.tempo.milli_quarters_per_minute(),
            )
        })
        .collect();
    // "slow" is two 4/4 bars of 20160 ticks, so "faster" starts at 40320.
    // "faster" is two more, so "waltz" starts at 80640: 3/4 at 120 beats/min
    // is still 120000 mq/min, no change. "waltz" is two 3/4 bars of 15120
    // ticks, so "six eight" starts at 110880: its dotted quarter beat at 120
    // beats/min is 180000 mq/min.
    assert_eq!(changes, vec![(40320, 120_000), (110_880, 180_000)]);
}

#[test]
fn a_repeated_section_makes_no_change() {
    // "six eight" plays twice at 180000 mq/min: one change for it, not two.
    assert_eq!(plan().tempo_changes.len(), 2);
}
