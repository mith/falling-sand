use bevy::{math::IVec2, utils::HashSet};

use crate::falling_sand_grid::FallingSandGrid;

/// Iterator for chunk positions in an extended checkerboard pattern.
pub struct ExtendedCheckerboardIterator<'a> {
    grid: &'a FallingSandGrid,
    iteration: i32,
}

impl<'a> Iterator for ExtendedCheckerboardIterator<'a> {
    type Item = HashSet<IVec2>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iteration >= 4 {
            return None; // End after four iterations
        }

        let mut set = HashSet::new();
        for &pos in self.grid.active_chunks().iter() {
            match self.iteration {
                0 => {
                    if pos.x % 2 == 0 && pos.y % 2 == 0 {
                        set.insert(pos);
                    }
                }
                1 => {
                    if pos.x % 2 == 1 && pos.y % 2 == 0 {
                        set.insert(pos);
                    }
                }
                2 => {
                    if pos.x % 2 == 0 && pos.y % 2 == 1 {
                        set.insert(pos);
                    }
                }
                3 => {
                    if pos.x % 2 == 1 && pos.y % 2 == 1 {
                        set.insert(pos);
                    }
                }
                _ => unreachable!(),
            }
        }

        self.iteration += 1; // Move to the next set in the next iteration

        if set.is_empty() {
            None
        } else {
            Some(set)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::material::Material;

    use super::*;

    #[test]
    fn test_extended_checkerboard_iterator() {
        let mut grid = FallingSandGrid::new();
        grid.set_particle(grid.chunk_size().x, 0, Material::Water);
        grid.set_particle(grid.chunk_size().x, grid.chunk_size().y, Material::Water);
        grid.set_particle(0, grid.chunk_size().y, Material::Water);
        let mut iter = ExtendedCheckerboardIterator {
            grid: &grid,
            iteration: 0,
        };

        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(0, 0)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(1, 0)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(0, 1)])));
        assert_eq!(iter.next(), Some(HashSet::from([IVec2::new(1, 1)])));
        assert_eq!(iter.next(), None);
    }
}
