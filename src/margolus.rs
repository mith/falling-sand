use bevy::prelude::*;
use ndarray::{arr2, ArrayView2, ArrayViewMut2, Zip};

use crate::types::Material;

#[derive(Resource, Default)]
pub struct MargulosState {
    pub odd_timestep: bool,
}

pub fn margolus_gravity(source: ArrayView2<Material>, target: ArrayViewMut2<Material>) {
    // Method from: https://ir.cwi.nl/pub/4545
    Zip::from(target.reversed_axes().exact_chunks_mut((2, 2)))
        .and(source.reversed_axes().exact_chunks((2, 2)))
        .par_for_each(|mut s, neigh| {
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
}
