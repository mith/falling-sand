use bevy::prelude::*;

use crate::grid::Board;
use crate::types::Material;

#[derive(Component)]
pub struct FallingSand {
    pub cells: Board,
    pub scratch: Board,
    pub texture: Handle<Image>,
    pub odd_timestep: bool,
}

impl FallingSand {
    pub fn new(width: usize, height: usize, texture: Handle<Image>) -> Self {
        FallingSand {
            cells: Board::new(width, height),
            scratch: Board::new(width, height),
            texture,
            odd_timestep: false,
        }
    }

    pub fn new_from_board(board: &Board, texture: Handle<Image>) -> Self {
        let width = board.nrows();
        let height = board.ncols();
        FallingSand {
            cells: board.clone(),
            scratch: Board::new(width, height),
            texture,
            odd_timestep: false,
        }
    }
}

pub fn grid_system(grid_query: Query<&FallingSand>, mut textures: ResMut<Assets<Image>>) {
    for grid in grid_query.iter() {
        if let Some(texture) = textures.get_mut(&grid.texture) {
            texture.data = grid
                .cells
                .t()
                .iter()
                .flat_map(|cell| match *cell {
                    Material::Sand => [244, 215, 21, 255u8],
                    Material::Water => [255, 0, 0, 255u8],
                    Material::Bedrock => [77, 77, 77, 255u8],
                    _ => [255u8, 255u8, 255u8, 255u8],
                })
                .collect();
        }
    }
}
