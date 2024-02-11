use bevy::{math::IVec2, utils::HashSet};

/// Iterator for iterating over active chunks in a sparse grid pattern.
/// The chunks in each returned set are guaranteed to be at least 2 chunks apart.
pub struct SparseGridIterator {
    active_chunks: HashSet<IVec2>,
    iteration: i32,
}

impl SparseGridIterator {
    pub fn new(active_chunks: HashSet<IVec2>) -> SparseGridIterator {
        // Clone the active chunks from the grid

        SparseGridIterator {
            active_chunks,
            iteration: 0,
        }
    }
}

impl Iterator for SparseGridIterator {
    type Item = HashSet<IVec2>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.iteration < 9 {
            let current_iteration = self.iteration;
            self.iteration += 1;
            let iteration_chunks: HashSet<IVec2> = self
                .active_chunks
                .iter()
                .filter(|pos| {
                    let set_index = chunk_pos_set_index(pos);
                    set_index == current_iteration
                })
                .copied()
                .collect();

            if !iteration_chunks.is_empty() {
                return Some(iteration_chunks);
            }
        }
        None
    }
}

fn chunk_pos_set_index(pos: &IVec2) -> i32 {
    let positive_mod = |n: i32, m: i32| ((n % m) + m) % m;
    let x = positive_mod(pos.x, 3);
    let y = positive_mod(pos.y, 3);
    x + y * 3
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_chunk_pos_set_index() {
        assert_eq!(chunk_pos_set_index(&IVec2::new(0, 0)), 0);
        assert_eq!(chunk_pos_set_index(&IVec2::new(1, 0)), 1);
        assert_eq!(chunk_pos_set_index(&IVec2::new(2, 0)), 2);
        assert_eq!(chunk_pos_set_index(&IVec2::new(0, 1)), 3);
        assert_eq!(chunk_pos_set_index(&IVec2::new(1, 1)), 4);
        assert_eq!(chunk_pos_set_index(&IVec2::new(2, 1)), 5);
        assert_eq!(chunk_pos_set_index(&IVec2::new(0, 2)), 6);
        assert_eq!(chunk_pos_set_index(&IVec2::new(1, 2)), 7);
        assert_eq!(chunk_pos_set_index(&IVec2::new(2, 2)), 8);
    }

    #[test]
    fn test_chunk_pos_set_index_negative() {
        assert_eq!(chunk_pos_set_index(&IVec2::new(-1, 0)), 2);
        assert_eq!(chunk_pos_set_index(&IVec2::new(-2, 0)), 1);
        assert_eq!(chunk_pos_set_index(&IVec2::new(-3, 0)), 0);
        assert_eq!(chunk_pos_set_index(&IVec2::new(0, -1)), 6);
        assert_eq!(chunk_pos_set_index(&IVec2::new(0, -2)), 3);
        assert_eq!(chunk_pos_set_index(&IVec2::new(0, -3)), 0);
    }

    #[test]
    fn test_extended_checkerboard_iterator() {
        let active_chunks = HashSet::from([
            IVec2::new(0, 0),
            IVec2::new(1, 0),
            IVec2::new(2, 0),
            IVec2::new(0, 1),
            IVec2::new(1, 1),
            IVec2::new(2, 1),
            IVec2::new(0, 2),
            IVec2::new(1, 2),
            IVec2::new(2, 2),
        ]);
        let mut iter = SparseGridIterator::new(active_chunks);

        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(0, 0)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(1, 0)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(2, 0)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(0, 1)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(1, 1)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(2, 1)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(0, 2)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(1, 2)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(2, 2)])));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_extended_checkerboard_iterator_skip_inactive_chunks() {
        let active_chunks = HashSet::from([
            IVec2::new(0, 0),
            IVec2::new(1, 0),
            IVec2::new(2, 0),
            IVec2::new(1, 1),
            IVec2::new(2, 1),
            IVec2::new(0, 2),
            IVec2::new(1, 2),
        ]);
        let mut iter = SparseGridIterator::new(active_chunks);

        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(0, 0)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(1, 0)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(2, 0)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(1, 1)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(2, 1)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(0, 2)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(1, 2)])));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_extended_checkerboard_iterator_2_chunks_between_returned_chunks() {
        let active_chunks = HashSet::from([IVec2::new(0, 0), IVec2::new(-3, 0), IVec2::new(3, 0)]);
        let mut iter = SparseGridIterator::new(active_chunks);

        assert_eq!(
            iter.next(),
            Some(HashSet::from([
                IVec2::new(0, 0),
                IVec2::new(-3, 0),
                IVec2::new(3, 0)
            ]))
        );
        assert_eq!(iter.next(), None);
    }
}
