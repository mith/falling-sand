use bevy::prelude::*;
use ndarray::{arr2, s, Zip};

use crate::falling_sand::FallingSand;
use crate::types::Material;

#[derive(Resource, Default)]
pub struct Margulos {
    pub odd_timestep: bool,
}

pub fn gravity_system(mut grid_query: Query<&mut FallingSand>, mut margolus: ResMut<Margulos>) {
    for mut grid in grid_query.iter_mut() {
        let grid = &mut *grid;
        let (source, target) = {
            if !margolus.odd_timestep {
                (grid.cells.view(), grid.scratch.view_mut())
            } else {
                (
                    grid.cells.slice(s![1..-1, 1..-1]),
                    grid.scratch.slice_mut(s![1..-1, 1..-1]),
                )
            }
        };

        // Method from: https://ir.cwi.nl/pub/4545

        Zip::from(target.reversed_axes().exact_chunks_mut((2, 2)))
            .and(source.reversed_axes().exact_chunks((2, 2)))
            .for_each(|mut s, neigh| {
                if neigh.iter().all(|material| *material == Material::Air) {
                    s.assign(&neigh);
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Air],
                        [Material::Air, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Air],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Air, Material::Sand],
                        [Material::Air, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Air, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Sand],
                        [Material::Air, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Air],
                        [Material::Air, Material::Sand],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Air, Material::Sand],
                        [Material::Sand, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Air],
                        [Material::Sand, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Air, Material::Sand],
                        [Material::Air, Material::Sand],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Sand],
                        [Material::Air, Material::Sand],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Sand],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Sand],
                        [Material::Sand, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Sand, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else {
                    s.assign(&neigh);
                }
            });
        grid.cells.assign(&grid.scratch);
        margolus.odd_timestep = !margolus.odd_timestep;
    }
}
