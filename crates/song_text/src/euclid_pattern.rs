//! The Euclidean rhythm `euclid(hits,steps)` (Song Notation, section 3):
//! `hits` hits spread as evenly as possible over `steps` steps, by
//! Bjorklund's algorithm, with the first step a hit. `euclid(3,8)` is hit,
//! rest, rest, hit, rest, rest, hit, rest.

/// For each step, whether it is a hit.
///
/// Bjorklund's method: start with one group per hit and one per rest, and
/// while more than one group is left over, join each leftover-free pair
/// (a hit group with a rest group) into one group. The pattern is the
/// groups in order.
///
/// The caller has checked `1 <= hits <= steps`.
#[must_use]
pub fn euclid_pattern(hits: usize, steps: usize) -> Vec<bool> {
    let mut front: Vec<Vec<bool>> = vec![vec![true]; hits];
    let mut back: Vec<Vec<bool>> = vec![vec![false]; steps.saturating_sub(hits)];
    while back.len() > 1 {
        let pairs: usize = front.len().min(back.len());
        let joined: Vec<Vec<bool>> = (0..pairs)
            .map(|index: usize| {
                let mut group: Vec<bool> = front[index].clone();
                group.extend(back[index].iter());
                group
            })
            .collect();
        // What is left over of the longer list becomes the new back.
        let leftover: Vec<Vec<bool>> = if front.len() > back.len() {
            front[pairs..].to_vec()
        } else {
            back[pairs..].to_vec()
        };
        front = joined;
        back = leftover;
    }
    front.into_iter().chain(back).flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `x` for a hit, `.` for a rest, as rhythm writers print it.
    fn drawn(hits: usize, steps: usize) -> String {
        euclid_pattern(hits, steps)
            .into_iter()
            .map(|hit: bool| if hit { 'x' } else { '.' })
            .collect()
    }

    #[test]
    fn the_standard_euclidean_patterns_come_out() {
        assert_eq!(drawn(3, 8), "x..x..x.");
        assert_eq!(drawn(5, 8), "x.xx.xx.");
        assert_eq!(drawn(2, 5), "x.x..");
        assert_eq!(drawn(4, 4), "xxxx");
        assert_eq!(drawn(1, 4), "x...");
        assert_eq!(drawn(7, 16), "x..x.x.x..x.x.x.");
    }

    #[test]
    fn every_pattern_has_the_hits_and_steps_asked_for_and_starts_on_a_hit() {
        for steps in 1..=16 {
            for hits in 1..=steps {
                let pattern: Vec<bool> = euclid_pattern(hits, steps);
                assert_eq!(pattern.len(), steps);
                assert_eq!(pattern.iter().filter(|&&hit: &&bool| hit).count(), hits);
                assert!(pattern[0], "euclid({hits},{steps}) must start on a hit");
            }
        }
    }
}
