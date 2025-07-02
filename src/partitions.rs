use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct PartitionClass {
    pub strict_winner_scores: Vec<u32>,
    pub augmentation_range: RangeInclusive<usize>,
}

impl PartitionClass {
    pub fn n_cases(&self) -> usize {
        1 + self.augmentation_range.end() - self.augmentation_range.start()
    }

    pub fn required(&self, n_repeats: usize) -> Vec<u32> {
        let mut scores = self.strict_winner_scores.clone();
        scores.extend(vec![self.strict_winner_scores.last().unwrap(); n_repeats]);
        scores
    }
}

fn fill_remaining(
    end_slice: &mut [u32],
    fill_value: u32,
    target_total: u32,
    n_elided_buckets: usize,
) -> RangeInclusive<usize> {
    let mut points_remaining = target_total;
    for k in 0..end_slice.len() {
        if points_remaining >= fill_value {
            end_slice[k] = fill_value;
            points_remaining -= fill_value
        } else if points_remaining > 0 {
            end_slice[k] = points_remaining;
            points_remaining = 0;
        } else {
            end_slice[k] = 0;
        }
    }
    if points_remaining == 0 {
        return 0..=0;
    }
    let t_min =
        points_remaining.saturating_sub((fill_value - 1) * (n_elided_buckets as u32)) as usize;
    let t_max = (points_remaining / fill_value) as usize;
    t_min..=t_max.min(n_elided_buckets)
}

pub fn partitions(n_competitors: usize, n_winners: usize) -> Vec<PartitionClass> {
    let mut results = Vec::new();
    let max_score = 2 * (n_competitors - 1) as u32;
    let total_score = (n_competitors * (n_competitors - 1)) as u32;
    let mut current_partition = vec![0; n_winners];
    // last nonzero element
    let mut cursor = 0;
    let mut fill_score = max_score;
    'outer: loop {
        let total_remaining = total_score - current_partition[..cursor].iter().sum::<u32>();
        let additional = fill_remaining(
            &mut current_partition[cursor..],
            fill_score,
            total_remaining,
            n_competitors - n_winners,
        );
        // println!("{:?}, {:?}", current_partition, adds);

        if *additional.start() > (n_competitors - n_winners) {
            while current_partition[cursor] <= 1 {
                if cursor == 0 {
                    break 'outer;
                }
                cursor -= 1;
            }
        } else {
            results.push(PartitionClass {
                strict_winner_scores: current_partition.clone(),
                augmentation_range: additional,
            });

            cursor = if let Some(k) = current_partition.iter().position(|&x| x <= 1) {
                k - 1
            } else {
                n_winners - 1
            };
        }

        fill_score = current_partition[cursor] - 1;
    }
    results
}

// def get_score_partitions(n_players, n_winners):
//     return sorted(
//         partitions(n_players, n_winners),
//         key=lambda p: (len(p), -len(set(p)), reversor(p)),
//     )

// class reversor:
//     def __init__(self, obj):
//         self.obj = obj

//     def __eq__(self, other):
//         return other.obj == self.obj

//     def __lt__(self, other):
//         return other.obj < self.obj
