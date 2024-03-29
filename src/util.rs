use bevy::math::IVec2;
use rand::{rngs::StdRng, Rng};

use crate::consts::CHUNK_SIZE;

pub fn positive_mod(a: i32, b: i32) -> i32 {
    (a % b + b) % b
}

pub fn random_dir_range(rng: &mut StdRng, min: i32, max: i32) -> Box<dyn Iterator<Item = i32>> {
    let reverse = rng.gen_bool(0.5);
    if reverse {
        Box::new((min..max).rev())
    } else {
        Box::new(min..max)
    }
}

#[inline(always)]
pub const fn below(IVec2 { x, y }: IVec2) -> IVec2 {
    IVec2 { x, y: y - 1 }
}

#[inline(always)]
pub const fn above(IVec2 { x, y }: IVec2) -> IVec2 {
    IVec2 { x, y: y + 1 }
}

#[inline(always)]
pub const fn left(IVec2 { x, y }: IVec2) -> IVec2 {
    IVec2 { x: x - 1, y }
}

#[inline(always)]
pub const fn right(IVec2 { x, y }: IVec2) -> IVec2 {
    IVec2 { x: x + 1, y }
}

#[inline(always)]
pub const fn below_left(IVec2 { x, y }: IVec2) -> IVec2 {
    IVec2 { x: x - 1, y: y - 1 }
}

#[inline(always)]
pub const fn below_right(IVec2 { x, y }: IVec2) -> IVec2 {
    IVec2 { x: x + 1, y: y - 1 }
}

pub fn chunk_neighbors(chunk_position: IVec2) -> [IVec2; 8] {
    [
        IVec2::new(chunk_position.x - 1, chunk_position.y - 1),
        IVec2::new(chunk_position.x, chunk_position.y - 1),
        IVec2::new(chunk_position.x + 1, chunk_position.y - 1),
        IVec2::new(chunk_position.x - 1, chunk_position.y),
        IVec2::new(chunk_position.x + 1, chunk_position.y),
        IVec2::new(chunk_position.x - 1, chunk_position.y + 1),
        IVec2::new(chunk_position.x, chunk_position.y + 1),
        IVec2::new(chunk_position.x + 1, chunk_position.y + 1),
    ]
}

pub fn chunk_neighbors_n(chunk_position: IVec2, n: i32) -> Vec<IVec2> {
    let mut neighbors = vec![];
    for x in -n..n + 1 {
        for y in -n..n + 1 {
            if x == 0 && y == 0 {
                continue;
            }
            neighbors.push(IVec2::new(chunk_position.x + x, chunk_position.y + y));
        }
    }
    neighbors
}
pub fn tile_pos_to_chunk_pos(IVec2 { x, y }: IVec2) -> IVec2 {
    let floor_div = |a: i32, b: i32| {
        if a < 0 && a % b != 0 {
            (a / b) - 1
        } else {
            a / b
        }
    };
    IVec2::new(floor_div(x, CHUNK_SIZE), floor_div(y, CHUNK_SIZE))
}
#[cfg(test)]
mod test {
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn test_tile_pos_to_chunk_pos() {
        assert_eq!(tile_pos_to_chunk_pos((0, 0).into()), IVec2::new(0, 0));
        assert_eq!(tile_pos_to_chunk_pos((63, 63).into()), IVec2::new(0, 0));
        assert_eq!(tile_pos_to_chunk_pos((64, 64).into()), IVec2::new(1, 1));
        assert_eq!(tile_pos_to_chunk_pos((65, 65).into()), IVec2::new(1, 1));
        assert_eq!(tile_pos_to_chunk_pos((0, -1).into()), IVec2::new(0, -1));
    }

    #[test]
    fn test_random_dir_range() {
        let mut rng = StdRng::seed_from_u64(0);
        let range = random_dir_range(&mut rng, 0, 10).collect::<Vec<_>>();
        assert_eq!(range, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_random_dir_range_reverse() {
        let mut rng = StdRng::seed_from_u64(2);
        let range = random_dir_range(&mut rng, 0, 10).collect::<Vec<_>>();
        assert_eq!(range, vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    }
}
