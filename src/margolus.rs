use bevy::prelude::*;
use ndarray::{arr2, ArrayView2, ArrayViewMut2, Zip};

use crate::types::Material;

#[derive(Resource, Default)]
pub struct MargulosState {
    pub odd_timestep: bool,
}

pub fn margolus_gravity(
    source: ArrayView2<Material>,
    target: ArrayViewMut2<Material>,
    parallel: bool,
) {
    const CHUNK_SIZE: (usize, usize) = (2, 2);
    if parallel {
        Zip::from(target.reversed_axes().exact_chunks_mut(CHUNK_SIZE))
            .and(source.reversed_axes().exact_chunks(CHUNK_SIZE))
            .par_for_each(margolus_gravity_neighborhood);
    } else {
        Zip::from(target.reversed_axes().exact_chunks_mut(CHUNK_SIZE))
            .and(source.reversed_axes().exact_chunks(CHUNK_SIZE))
            .for_each(margolus_gravity_neighborhood);
    }
}

fn margolus_gravity_neighborhood(
    mut target: ArrayViewMut2<Material>,
    source: ArrayView2<Material>,
) {
    // Method from: https://ir.cwi.nl/pub/4545
    if source.iter().all(|material| *material == Material::Air) {
        target.assign(&source);
    } else if source
        == arr2(&[
            [Material::Sand, Material::Air],
            [Material::Air, Material::Air],
        ])
    {
        target.assign(&arr2(&[
            [Material::Air, Material::Air],
            [Material::Sand, Material::Air],
        ]));
    } else if source
        == arr2(&[
            [Material::Air, Material::Sand],
            [Material::Air, Material::Air],
        ])
    {
        target.assign(&arr2(&[
            [Material::Air, Material::Air],
            [Material::Air, Material::Sand],
        ]));
    } else if source
        == arr2(&[
            [Material::Sand, Material::Sand],
            [Material::Air, Material::Air],
        ])
    {
        target.assign(&arr2(&[
            [Material::Air, Material::Air],
            [Material::Sand, Material::Sand],
        ]));
    } else if source
        == arr2(&[
            [Material::Sand, Material::Air],
            [Material::Air, Material::Sand],
        ])
    {
        target.assign(&arr2(&[
            [Material::Air, Material::Air],
            [Material::Sand, Material::Sand],
        ]));
    } else if source
        == arr2(&[
            [Material::Air, Material::Sand],
            [Material::Sand, Material::Air],
        ])
    {
        target.assign(&arr2(&[
            [Material::Air, Material::Air],
            [Material::Sand, Material::Sand],
        ]));
    } else if source
        == arr2(&[
            [Material::Sand, Material::Air],
            [Material::Sand, Material::Air],
        ])
    {
        target.assign(&arr2(&[
            [Material::Air, Material::Air],
            [Material::Sand, Material::Sand],
        ]));
    } else if source
        == arr2(&[
            [Material::Air, Material::Sand],
            [Material::Air, Material::Sand],
        ])
    {
        target.assign(&arr2(&[
            [Material::Air, Material::Air],
            [Material::Sand, Material::Sand],
        ]));
    } else if source
        == arr2(&[
            [Material::Sand, Material::Sand],
            [Material::Air, Material::Sand],
        ])
    {
        target.assign(&arr2(&[
            [Material::Air, Material::Sand],
            [Material::Sand, Material::Sand],
        ]));
    } else if source
        == arr2(&[
            [Material::Sand, Material::Sand],
            [Material::Sand, Material::Air],
        ])
    {
        target.assign(&arr2(&[
            [Material::Sand, Material::Air],
            [Material::Sand, Material::Sand],
        ]));
    } else {
        target.assign(&source);
    }
}
