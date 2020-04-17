use std::convert::{TryFrom, TryInto};

#[derive(Debug, Eq, PartialEq)]
pub enum Diff {
    Insert { index: usize },
    Delete { index: usize },
}

pub fn diff<T: Eq>(old: &[T], new: &[T]) -> Vec<Diff> {
    let mut diff = Vec::new();
    diff_impl(&mut diff, old, new, 0, 0);
    diff
}

fn diff_impl<T: Eq>(
    diff: &mut Vec<Diff>,
    old: &[T],
    new: &[T],
    offset_old: usize,
    offset_new: usize,
) {
    fn split<T>(slice: &[T], idx1: usize, idx2: usize) -> (&[T], &[T]) {
        let len = slice.len();
        let (lhs, rhs) = if idx2 >= len {
            (slice, &[] as &[T])
        } else {
            slice.split_at(idx2)
        };

        if idx1 == idx2 {
            (lhs, rhs)
        } else {
            (lhs.split_at(idx1.min(lhs.len() - 1)).0, rhs)
        }
    }

    let num_old = old.len();
    assert!(
        isize::try_from(num_old).is_ok(),
        "The diff algorithm only supports `Vec` sizes up to isize::MAX"
    );
    let num_new = new.len();
    assert!(
        isize::try_from(num_new).is_ok(),
        "The diff algorithm only supports `Vec` sizes up to isize::MAX"
    );
    if num_old > 0 && num_new > 0 {
        let v_size = 2 * num_old.min(num_new) + 2;
        assert!(
            isize::try_from(v_size).is_ok(),
            "The diff algorithm only supports combined entry sizes up to isize::MAX"
        );

        let mut v_forward = vec![0usize; v_size];
        let mut v_backward = vec![0usize; v_size];
        let v_size: isize = v_size as isize;

        let max = num_old + num_new;
        let delta = num_old as isize - num_new as isize;
        for half_d in 0..=(max / 2 + max % 2) {
            let half_d = half_d as isize;
            let left_bound = -half_d + 2 * 0.max(half_d - num_new as isize);
            let right_bound = half_d - 2 * 0.max(half_d - num_old as isize);
            for is_forward in &[true, false] {
                let (v1, v2, oddity, sign) = if *is_forward {
                    (&mut v_forward, &v_backward, 1isize, 1isize)
                } else {
                    (&mut v_backward, &v_forward, 0isize, -1isize)
                };
                for k in (left_bound..=right_bound).step_by(2) {
                    let mut x = if k == -half_d
                        || (k != half_d
                            && v1[(k - 1).rem_euclid(v_size) as usize]
                                < v1[(k + 1).rem_euclid(v_size) as usize])
                    {
                        v1[(k + 1).rem_euclid(v_size) as usize]
                    } else {
                        v1[(k - 1).rem_euclid(v_size) as usize] + 1
                    };
                    let mut y = (x as isize - k) as usize;
                    let x_start = x;
                    let y_start = y;
                    while x < num_old
                        && y < num_new
                        && old[((1 - oddity) * (num_old as isize - 1) + sign * x as isize) as usize]
                            == new[((1 - oddity) * (num_new as isize - 1) + sign * y as isize)
                                as usize]
                    {
                        x += 1;
                        y += 1;
                    }
                    v1[k.rem_euclid(v_size) as usize] = x;
                    let inverse_k = -k + delta;
                    if max % 2 == oddity as usize
                        && (inverse_k >= -half_d + oddity)
                        && (inverse_k <= half_d - oddity)
                        && v1[k.rem_euclid(v_size) as usize]
                            + v2[inverse_k.rem_euclid(v_size) as usize]
                            >= num_old
                    {
                        let d = 2 * half_d - oddity;
                        let (x1, y1, x2, y2) = if *is_forward {
                            (x_start, y_start, x, y)
                        } else {
                            (
                                num_old - x,
                                num_new - y,
                                num_old - x_start,
                                num_new - y_start,
                            )
                        };
                        if d > 1 || (x1 != x2 && y1 != y2) {
                            let (old_lhs, old_rhs) = split(old, x1, x2);
                            let (new_lhs, new_rhs) = split(new, y1, y2);
                            diff_impl(diff, old_lhs, new_lhs, offset_old, offset_new);
                            diff_impl(diff, old_rhs, new_rhs, offset_old + x2, offset_new + y2);
                        } else if num_new > num_old {
                            let (_, rhs) = new.split_at(num_old);
                            diff_impl(diff, &[], rhs, offset_old + num_old, offset_new + num_old);
                        } else if num_new < num_old {
                            let (_, rhs) = old.split_at(num_new);
                            diff_impl(diff, rhs, &[], offset_old + num_new, offset_new + num_new);
                        }
                        return;
                    }
                }
            }
        }
    } else if num_old > 0 {
        for idx in 0..num_old {
            diff.push(Diff::Delete {
                index: offset_old + idx,
            });
        }
    } else {
        for idx in 0..num_new {
            diff.push(Diff::Insert {
                index: offset_new + idx,
            })
        }
    }
}

pub fn diff_length<T: Eq>(old: &[T], new: &[T]) -> usize {
    let num_old = old.len();
    assert!(
        isize::try_from(num_old).is_ok(),
        "The diff algorithm only supports `Vec` sizes up to isize::MAX"
    );
    let num_new = new.len();
    assert!(
        isize::try_from(num_new).is_ok(),
        "The diff algorithm only supports `Vec` sizes up to isize::MAX"
    );
    let max: isize = (num_old + num_new)
        .try_into()
        .expect("The diff algorithm only supports combined entry sizes up to isize::MAX");

    let mut v = vec![0usize; 2 * max as usize + 2];
    for d in 0..=max {
        let left_bound = -d;
        let right_bound = d;
        for k in (left_bound..=right_bound).step_by(2) {
            let idx: usize = (k + max).try_into().unwrap();
            let mut x = if k == -d || (k != d && v[idx - 1] < v[idx + 1]) {
                v[idx + 1]
            } else {
                v[idx - 1] + 1
            };
            let mut y = (x as isize - k) as usize;
            while x < num_old && y < num_new && old[x] == new[y] {
                x += 1;
                y += 1;
            }
            v[idx] = x;
            if x == num_old && y == num_new {
                return d as usize;
            }
        }
    }

    unreachable!()
}

pub fn split_diff(diff: &[Diff]) -> (Vec<usize>, Vec<usize>) {
    let deletions = diff
        .iter()
        .filter_map(|diff| match diff {
            Diff::Delete { index } => Some(*index),
            _ => None,
        })
        .collect();
    let insertions = diff
        .iter()
        .filter_map(|diff| match diff {
            Diff::Insert { index } => Some(*index),
            _ => None,
        })
        .collect();

    (deletions, insertions)
}
