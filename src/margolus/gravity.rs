use bevy::ecs::system::{Query, Res};
use ndarray::{arr2, ArrayView2, ArrayViewMut2};

use crate::{
    falling_sand::FallingSandGrid,
    types::{MaterialDensities, MaterialStates, Particle, StateOfMatter},
};

use super::{margulos, MargolusSettings, MargulosState};

pub fn margolus_gravity(
    mut grid_query: Query<&mut FallingSandGrid>,
    margolus_state: Res<MargulosState>,
    margulos_settings: Res<MargolusSettings>,
    material_densities: Res<MaterialDensities>,
    material_states: Res<MaterialStates>,
) {
    for mut grid in grid_query.iter_mut() {
        margulos(
            &margolus_state,
            &margulos_settings,
            &mut grid,
            |mut target, source| {
                margolus_gravity_neighborhood(
                    target.view_mut(),
                    source.view(),
                    &material_densities,
                    &material_states,
                )
            },
        );
    }
}

fn is_fluid(state: StateOfMatter) -> bool {
    matches!(state, StateOfMatter::Liquid | StateOfMatter::Gas)
}

fn margolus_gravity_neighborhood(
    mut target: ArrayViewMut2<Particle>,
    source: ArrayView2<Particle>,
    material_densities: &MaterialDensities,
    material_phases: &MaterialStates,
) {
    // Neighborhood:
    // a, b
    // c, d

    // Indexes for the 4 cells in the neighborhood
    let a_i = (0, 0);
    let b_i = (0, 1);
    let c_i = (1, 0);
    let d_i = (1, 1);

    // Get the 4 cells in the neighborhood
    let a = *source.get(a_i).unwrap();
    let b = *source.get(b_i).unwrap();
    let c = *source.get(c_i).unwrap();
    let d = *source.get(d_i).unwrap();

    let a_density = material_densities[a.material];
    let b_density = material_densities[b.material];
    let c_density = material_densities[c.material];
    let d_density = material_densities[d.material];

    let a_phase = material_phases[a.material];
    let b_phase = material_phases[b.material];
    let c_phase = material_phases[c.material];
    let d_phase = material_phases[d.material];

    if source.iter().all(|p| p.material == source[[0, 0]].material) {
        // If all cells match, just copy the source to the target
        // Since this is the most common case, it's worth checking for
        // before doing the more expensive checks below
        target.assign(&source);
    } else if source
        .iter()
        .map(|p| material_phases[p.material])
        .all(is_fluid)
        && a_density > c_density
        && b_density > d_density
    {
        // If all cells are fluid and top ones are heavier than the bottom ones,
        // swap the top ones with the bottom ones
        target.assign(&arr2(&[[c, d], [a, b]]));
    } else if is_fluid(a_phase) && is_fluid(c_phase) && a_density > c_density {
        // If the top left cell is fluid and heavier than the bottom left cell,
        // swap the top left cell with the bottom left cell
        target.assign(&arr2(&[[c, b], [a, d]]));
    } else if is_fluid(b_phase) && is_fluid(d_phase) && b_density > d_density {
        // If the top right cell is fluid and heavier than the bottom right cell,
        // swap the top right cell with the bottom right cell
        target.assign(&arr2(&[[a, d], [c, b]]));
    } else if is_fluid(a_phase) && is_fluid(d_phase) && a_density > d_density {
        // If the top left cell is fluid and heavier than the bottom right cell,
        // swap the top left cell with the bottom right cell
        target.assign(&arr2(&[[d, b], [c, a]]));
    } else if is_fluid(b_phase) && is_fluid(c_phase) && b_density > c_density {
        // If the top right cell is fluid and heavier than the bottom left cell,
        // swap the top right cell with the bottom left cell
        target.assign(&arr2(&[[a, c], [d, b]]));
    } else {
        // Otherwise, just copy the source to the target
        target.assign(&source);
    }
}
