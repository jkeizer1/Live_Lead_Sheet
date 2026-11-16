//! The loudness a `dynamics` line gives along a section (Song Notation,
//! section 5, "Dynamics, in rhythm notation").
//!
//! A level holds until the next mark. A crescendo or diminuendo moves the
//! velocity in a straight line in time, from the level before it to the next
//! level written, which may be bars later. A section starts at `mf` (the
//! track's normal velocity) and holds its last level to its end.

use vocabulary::dynamics::DynamicMark;
use vocabulary::midi_values::Velocity;
use vocabulary::song::{ElementContents, Line, LineElement};

/// One stretch of the section's loudness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment {
    /// From `start_ticks` on, this velocity, until the next segment.
    Level {
        start_ticks: u64,
        velocity: Velocity,
    },
    /// From `start_ticks` to `end_ticks`, a straight line from `from` to `to`.
    Ramp {
        start_ticks: u64,
        end_ticks: u64,
        from: Velocity,
        to: Velocity,
    },
}

impl Segment {
    const fn start_ticks(self) -> u64 {
        match self {
            Segment::Level { start_ticks, .. } | Segment::Ramp { start_ticks, .. } => start_ticks,
        }
    }
}

/// The velocity at any place of a section, from its dynamics line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicsTimeline {
    normal: Velocity,
    segments: Vec<Segment>,
}

impl DynamicsTimeline {
    /// A section with no dynamics line: `mf` throughout.
    #[must_use]
    pub fn flat(normal: Velocity) -> DynamicsTimeline {
        DynamicsTimeline {
            normal,
            segments: Vec::new(),
        }
    }

    /// The timeline of a section's dynamics `line`, for a track whose normal
    /// velocity (`mf`) is `normal`. The line has been checked when the song
    /// was loaded: every crescendo or diminuendo is followed by a level.
    #[must_use]
    pub fn from_line(line: &Line, normal: Velocity) -> DynamicsTimeline {
        let elements: &[LineElement] = line.elements();
        let mut segments: Vec<Segment> = Vec::new();
        // The level in force, which a crescendo or diminuendo starts from.
        let mut current: Velocity = normal;
        for (index, line_element) in elements.iter().enumerate() {
            let Some(ElementContents::Dynamic(mark)) = line_element.contents() else {
                // `r`: no change.
                continue;
            };
            let start_ticks: u64 = line_element.element().start().ticks();
            match mark {
                DynamicMark::Level(level) => {
                    current = level.velocity(normal);
                    segments.push(Segment::Level {
                        start_ticks,
                        velocity: current,
                    });
                }
                DynamicMark::Crescendo | DynamicMark::Diminuendo => {
                    // It ends where the next level mark begins.
                    let target: Option<(u64, Velocity)> =
                        elements[index + 1..]
                            .iter()
                            .find_map(|later: &LineElement| match later.contents() {
                                Some(ElementContents::Dynamic(DynamicMark::Level(level))) => {
                                    Some((later.element().start().ticks(), level.velocity(normal)))
                                }
                                _ => None,
                            });
                    if let Some((end_ticks, to)) = target {
                        segments.push(Segment::Ramp {
                            start_ticks,
                            end_ticks,
                            from: current,
                            to,
                        });
                    }
                    // With no target (refused when the song is loaded) the
                    // level in force simply holds.
                }
            }
        }
        DynamicsTimeline { normal, segments }
    }

    /// The track's normal velocity, `mf`: what the section starts at.
    #[must_use]
    pub const fn normal(&self) -> Velocity {
        self.normal
    }

    /// The velocity in force at `position_ticks` from the start of the
    /// section, rounded to the nearest whole velocity.
    #[must_use]
    pub fn velocity_at(&self, position_ticks: u64) -> Velocity {
        let in_force: Option<&Segment> = self
            .segments
            .iter()
            .rev()
            .find(|segment: &&Segment| segment.start_ticks() <= position_ticks);
        match in_force {
            None => self.normal,
            Some(&Segment::Level { velocity, .. }) => velocity,
            Some(&Segment::Ramp {
                start_ticks,
                end_ticks,
                from,
                to,
            }) => {
                if position_ticks >= end_ticks {
                    return to;
                }
                straight_line(
                    from,
                    to,
                    position_ticks - start_ticks,
                    end_ticks - start_ticks,
                )
            }
        }
    }
}

/// The velocity `elapsed_ticks` of the way through `length_ticks` on the
/// straight line from `from` to `to`, rounded half up.
///
/// velocity + (velocity difference × ticks ÷ ticks): the ticks cancel.
fn straight_line(from: Velocity, to: Velocity, elapsed_ticks: u64, length_ticks: u64) -> Velocity {
    let from_value: i64 = i64::from(from.value());
    let difference: i64 = i64::from(to.value()) - from_value;
    // Both are far below i64::MAX (a section is a few thousand bars at most).
    let elapsed: i64 = i64::try_from(elapsed_ticks).unwrap_or(i64::MAX);
    let length: i64 = i64::try_from(length_ticks).unwrap_or(i64::MAX).max(1);
    // Half up: floor((2 × numerator + length) ÷ (2 × length)).
    let numerator: i128 = i128::from(difference) * i128::from(elapsed);
    let moved: i128 = (2 * numerator + i128::from(length)).div_euclid(2 * i128::from(length));
    let value: i128 = i128::from(from_value) + moved;
    let low: i128 = i128::from(from.value().min(to.value()));
    let high: i128 = i128::from(from.value().max(to.value()));
    let clamped: u8 = u8::try_from(value.clamp(low, high)).unwrap_or(to.value());
    // `clamped` lies between two valid velocities, so it is valid; the
    // target is the harmless fallback for a value that cannot occur.
    Velocity::new(clamped).unwrap_or(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn velocity(value: u8) -> Velocity {
        Velocity::new(value).unwrap()
    }

    #[test]
    fn halfway_between_p_and_f_at_normal_64_is_64() {
        // p is 40, f is 88 at normal 64 (Song Notation, section 8).
        assert_eq!(
            straight_line(velocity(40), velocity(88), 1, 2),
            velocity(64)
        );
    }

    #[test]
    fn a_diminuendo_falls_in_a_straight_line() {
        assert_eq!(
            straight_line(velocity(88), velocity(40), 1, 4),
            velocity(76)
        );
        assert_eq!(
            straight_line(velocity(88), velocity(40), 4, 4),
            velocity(40)
        );
    }

    #[test]
    fn a_flat_timeline_is_normal_everywhere() {
        let timeline: DynamicsTimeline = DynamicsTimeline::flat(velocity(64));
        assert_eq!(timeline.velocity_at(0), velocity(64));
        assert_eq!(timeline.velocity_at(100_000), velocity(64));
    }
}
