use bevy::prelude::*;
use ndarray::{s, Array2, Zip};

use crate::{falling_sand::FallingSandGrid, types::Material, types::Particle};

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

pub fn flow(mut grid_query: Query<&mut FallingSandGrid>) {
    for mut grid in &mut grid_query {
        let mut flow_map =
            Array2::from_shape_simple_fn(grid.0.source().raw_dim(), || Flow::default());

        // Calculate outgoing flow for each particle
        for (i, particle) in grid.0.target().indexed_iter() {
            if particle.material != Material::Water {
                continue;
            }

            let mut remaining = particle.pressure;

            let mut flow_down = 0.;
            if i.1 > 0 {
                let below = &grid.0.target()[[i.0, i.1 - 1]];
                if below.material == Material::Water {
                    let pressure_cap = MAX_PRESSURE.max(particle.pressure + MAX_COMPRESSION);
                    flow_down = (pressure_cap - below.pressure).clamp(0., remaining);
                    remaining -= flow_down;
                }
            }

            let mut flow_left = 0.;
            if i.0 > 0 {
                let left = &grid.0.target()[[i.0 - 1, i.1]];
                if matches!(left.material, Material::Water | Material::Air) {
                    flow_left = (remaining - left.pressure).max(0.) * VISCOSITY;
                }
            }

            let mut flow_right = 0.;
            if i.0 < grid.0.source().dim().0 - 1 {
                let right = &grid.0.target()[[i.0 + 1, i.1]];
                if matches!(right.material, Material::Water | Material::Air) {
                    flow_right = (remaining - right.pressure).max(0.) * VISCOSITY;
                }
            }

            remaining -= flow_left + flow_right;

            let mut flow_up = 0.;
            if i.1 < grid.0.source().dim().1 - 1 {
                let above = &grid.0.target()[[i.0, i.1 + 1]];
                if matches!(above.material, Material::Water | Material::Air) {
                    let pressure_cap = MAX_PRESSURE.max(particle.pressure + MAX_COMPRESSION);
                    flow_up = (pressure_cap - above.pressure).clamp(0., remaining);
                }
            }

            flow_map[[i.0, i.1]] = Flow {
                up: flow_up,
                down: flow_down,
                left: flow_left,
                right: flow_right,
            };
        }

        let size = grid.0.source().dim();

        // Apply flow
        for (i, particle) in grid.0.target_mut().indexed_iter_mut() {
            if !matches!(particle.material, Material::Water | Material::Air) {
                return;
            }

            let flow = &flow_map[i];

            // subtract outgoing flow from current particle
            particle.pressure -= flow.up + flow.down + flow.left + flow.right;

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

            if particle.pressure > MIN_PRESSURE {
                particle.material = Material::Water;
            } else {
                particle.material = Material::Air;
            }
        }
    }
}
