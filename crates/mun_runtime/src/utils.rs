use std::cmp;

/// The Levenshtein distance is a string metric for measuring the difference between two sequences
/// A distance between two words is the minimum number of single-character edits
/// (insertions, deletions or substitutions) required to change one word into the other
pub fn lev_distance(a: &str, b: &str) -> usize {
    // cases which don't require further computation
    if a.is_empty() {
        return b.chars().count();
    } else if b.is_empty() {
        return a.chars().count();
    }

    let mut dcol: Vec<_> = (0..=b.len()).collect();
    let mut t_last = 0;

    for (i, sc) in a.chars().enumerate() {
        let mut current = i;
        dcol[0] = current + 1;

        for (j, tc) in b.chars().enumerate() {
            let next = dcol[j + 1];
            if sc == tc {
                dcol[j + 1] = current;
            } else {
                dcol[j + 1] = cmp::min(current, next);
                dcol[j + 1] = cmp::min(dcol[j + 1], dcol[j]) + 1;
            }
            current = next;
            t_last = j;
        }
    }
    dcol[t_last + 1]
}

#[cfg(test)]
mod tests {
    use crate::utils::lev_distance;

    #[test]
    fn distance_exists() {
        const FIRST_STRING: &str = "foo";
        const SECOND_STRING: &str = "zbar";
        const EXPECTED_DISTANCE: usize = 4;
        assert_eq!(lev_distance(FIRST_STRING, SECOND_STRING), EXPECTED_DISTANCE);
    }

    #[test]
    fn empty_string() {
        const FIRST_STRING: &str = "calculate";
        const SECOND_STRING: &str = "";
        const EXPECTED_DISTANCE: usize = FIRST_STRING.len();
        assert_eq!(lev_distance(FIRST_STRING, SECOND_STRING), EXPECTED_DISTANCE);
    }

    #[test]
    fn distance_is_zero() {
        const FIRST_STRING: &str = "calculate";
        const SECOND_STRING: &str = "calculate";
        const EXPECTED_DISTANCE: usize = 0;
        assert_eq!(lev_distance(FIRST_STRING, SECOND_STRING), EXPECTED_DISTANCE);
    }
}
