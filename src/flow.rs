use std::ops::Bound;

use bevy::prelude::*;
use enum_map::EnumMap;
use ndarray::{arr2, s, Array2, Zip};

use crate::{
    falling_sand::FallingSandGrid,
    types::Particle,
    types::{Material, MaterialDensities},
};

#[derive(Resource, Deref)]
pub struct MaterialFlowing(pub EnumMap<Material, bool>);

#[derive(Default)]
struct Flow {
    up: f32,
    down: f32,
    left: f32,
    right: f32,
}

pub const MIN_PRESSURE: f32 = 0.001;
pub const MAX_PRESSURE: f32 = 1.;
pub const MAX_COMPRESSION: f32 = 0.02;
pub const VISCOSITY: f32 = 0.33;

pub fn internal_flow(
    mut grid_query: Query<&mut FallingSandGrid>,
    material_flowing: Res<MaterialFlowing>,
) {
    let is_flowing = |particle: &Particle| material_flowing[particle.material];
    for mut grid in &mut grid_query {
        let mut flow_map =
            Array2::from_shape_simple_fn(grid.particles.source().raw_dim(), Flow::default);

        // Calculate outgoing flow for each particle
        for (i, particle) in grid.particles.source().indexed_iter() {
            if !is_flowing(particle) {
                continue;
            }

            let is_same_material = |other: &Particle| other.material == particle.material;

            let mut remaining = particle.pressure;

            let mut flow_down = 0.;
            if i.1 > 0 {
                let below = &grid.particles.source()[[i.0, i.1 - 1]];
                if is_same_material(below) {
                    let pressure_cap = MAX_PRESSURE.max(particle.pressure + MAX_COMPRESSION);
                    flow_down = (pressure_cap - below.pressure).clamp(0., remaining);
                    remaining -= flow_down;
                }
            }

            let mut flow_left = 0.;
            if i.0 > 0 {
                let left = &grid.particles.source()[[i.0 - 1, i.1]];
                if is_same_material(left) {
                    flow_left = (remaining - left.pressure).max(0.) * VISCOSITY;
                }
            }

            let mut flow_right = 0.;
            if i.0 < grid.particles.source().dim().0 - 1 {
                let right = &grid.particles.source()[[i.0 + 1, i.1]];
                if is_same_material(right) {
                    flow_right = (remaining - right.pressure).max(0.) * VISCOSITY;
                }
            }

            remaining -= flow_left + flow_right;

            let mut flow_up = 0.;
            if i.1 < grid.particles.source().dim().1 - 1 {
                let above = &grid.particles.source()[[i.0, i.1 + 1]];
                if is_same_material(above) {
                    let pressure_cap = MAX_PRESSURE.max(particle.pressure - MAX_COMPRESSION);
                    flow_up = (pressure_cap - above.pressure).clamp(0., remaining) * VISCOSITY;
                }
            }

            flow_map[[i.0, i.1]] = Flow {
                up: flow_up,
                down: flow_down,
                left: flow_left,
                right: flow_right,
            };
        }

        let size = grid.particles.source().dim();

        // Apply flow
        for (i, particle) in grid.particles.source_mut().indexed_iter_mut() {
            if !is_flowing(particle) {
                continue;
            }

            let flow = &flow_map[i];

            // subtract outgoing flow from current particle
            particle.pressure =
                (particle.pressure - flow.up + flow.down + flow.left + flow.right).max(0.);

            #[cfg(debug_assertions)]
            {
                if particle.pressure < 0. {
                    panic!("Negative pressure");
                }
            }

            // add incoming flow from adjacent particles
            if i.1 < size.1 - 1 {
                particle.pressure += flow_map[[i.0, i.1 + 1]].down;
            }
            if i.1 > 0 {
                particle.pressure += flow_map[[i.0, i.1 - 1]].up;
            }
            if i.0 > 0 {
                particle.pressure += flow_map[[i.0 - 1, i.1]].right;
            }
            if i.0 < size.0 - 1 {
                particle.pressure += flow_map[[i.0 + 1, i.1]].left;
            }
        }
    }
}

#[derive(Default)]
pub struct BoundaryFlowState {
    timestep: u32,
}

pub fn boundary_flow(
    mut grid_query: Query<&mut FallingSandGrid>,
    material_flowing: Res<MaterialFlowing>,
    material_densities: Res<MaterialDensities>,
    mut boundary_flow_state: Local<BoundaryFlowState>,
) {
    let is_flowing = |particle: &Particle| material_flowing[particle.material];
    const CHUNK_SIZE: (usize, usize) = (1, 3);
    for mut grid in &mut grid_query {
        let (source, target) = grid.source_and_target_mut();
        let reversed_target = &mut target.view_mut().reversed_axes();
        let reversed_source = &source.view().reversed_axes();
        let step = boundary_flow_state.timestep as usize;
        let sliced_source = reversed_source.slice(s![.., step..]);
        let mut sliced_target = reversed_target.slice_mut(s![.., step..]);
        Zip::from(sliced_source.exact_chunks(CHUNK_SIZE))
            .and(sliced_target.exact_chunks_mut(CHUNK_SIZE))
            .par_for_each(|source_chunk, target_chunk| {
                boundary_flow_neighborhood(
                    is_flowing,
                    &material_densities,
                    target_chunk,
                    source_chunk,
                );
            });

        grid.particles.swap();
    }

    boundary_flow_state.timestep = (boundary_flow_state.timestep + 1) % 3;
}

fn boundary_flow_neighborhood(
    is_flowing: impl Fn(&Particle) -> bool,
    material_densities: &MaterialDensities,
    mut target: ndarray::prelude::ArrayBase<
        ndarray::ViewRepr<&mut Particle>,
        ndarray::prelude::Dim<[usize; 2]>,
    >,
    source: ndarray::prelude::ArrayBase<
        ndarray::ViewRepr<&Particle>,
        ndarray::prelude::Dim<[usize; 2]>,
    >,
) {
    // Neighborhood:
    // a, b, c

    // Indexes for the 3 cells in the neighborhood
    let a_i = (0, 0);
    let b_i = (0, 1);
    let c_i = (0, 2);

    // Get the 3 cells in the neighborhood
    let a = *source.get(a_i).unwrap();
    let b = *source.get(b_i).unwrap();
    let c = *source.get(c_i).unwrap();

    // Multiply pressure by density so that denser materials push less dense materials
    let a_pressure_density = a.pressure * material_densities[a.material] as f32;
    let b_pressure_density = b.pressure * material_densities[b.material] as f32;
    let c_pressure_density = c.pressure * material_densities[c.material] as f32;

    if !is_flowing(&a)
        || !is_flowing(&b)
        || !is_flowing(&c)
        || a.material == b.material && b.material == c.material
    {
        target.assign(&source);
    } else if b.material == c.material
        && a_pressure_density > (c_pressure_density + b_pressure_density) / 2.
    {
        let new_a_b_pressure = a.pressure / 2.;
        let new_c_pressure = c.pressure + b.pressure;

        let new_a = Particle {
            material: a.material,
            pressure: new_a_b_pressure,
            velocity: a.velocity,
        };

        let new_b = Particle {
            material: a.material,
            pressure: new_a_b_pressure,
            velocity: b.velocity,
        };

        let new_c = Particle {
            material: c.material,
            pressure: new_c_pressure,
            velocity: c.velocity,
        };

        target.assign(&arr2(&[[new_a, new_b, new_c]]));
    } else if a.material == b.material
        && c_pressure_density > (a_pressure_density + b_pressure_density) / 2.
    {
        let new_c_b_pressure = c.pressure / 2.;
        let new_a_pressure = a.pressure + b.pressure;

        let new_a = Particle {
            material: a.material,
            pressure: new_a_pressure,
            velocity: a.velocity,
        };

        let new_b = Particle {
            material: c.material,
            pressure: new_c_b_pressure,
            velocity: b.velocity,
        };

        let new_c = Particle {
            material: c.material,
            pressure: new_c_b_pressure,
            velocity: c.velocity,
        };

        target.assign(&arr2(&[[new_a, new_b, new_c]]));
    } else {
        target.assign(&source);
    }
}
