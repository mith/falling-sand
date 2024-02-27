use rand::Rng;

use bevy::{ecs::system::Res, log::info_span, math::IVec2};

use crate::{
    chunk,
    falling_sand_grid::ChunkNeighborhoodView,
    material::{MaterialDensities, MaterialFlowing, MaterialStates, StateOfMatter},
    process_chunks::{process_chunks, ChunksParam},
    util::{below, below_left, below_right, left, random_dir_range, right},
};

pub fn fall(
    mut grid: ChunksParam,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
) {
    process_chunks(&mut grid, |chunk_pos, grid| {
        fall_chunk(grid, chunk_pos, &material_states, &material_densities)
    });
}

fn fall_chunk(
    grid: &mut ChunkNeighborhoodView,
    chunk_pos: IVec2,
    material_states: &MaterialStates,
    material_densities: &MaterialDensities,
) {
    let span = info_span!("fall_chunk");
    let _guard = span.enter();
    let chunk_size = grid.chunk_size();
    let min_y = chunk_size.y;
    let max_y = chunk_size.y * 2;
    for y in min_y..max_y {
        let min_x = chunk_size.x;
        let max_x = chunk_size.x * 2;
        let random_dir_range = {
            let rng = grid.center_chunk_mut().rng();
            random_dir_range(rng, min_x, max_x)
        };
        for x in random_dir_range {
            let particle_position = IVec2::new(x, y);
            let particle = *grid.get_particle(particle_position);
            let particle_is_dirty: bool = grid.get_dirty(particle_position);
            if material_states[particle.material] == StateOfMatter::Solid || particle_is_dirty {
                continue;
            }

            let mut is_eligible_particle = |other_particle_position| {
                can_fall_into(
                    grid,
                    other_particle_position,
                    material_states,
                    particle,
                    material_densities,
                )
            };

            let particle_below_position = below(particle_position);
            if is_eligible_particle(particle_below_position) {
                grid.swap_particles(particle_position, particle_below_position);
                grid.center_chunk_mut()
                    .attributes_mut()
                    .velocity
                    .set(particle.id, IVec2::NEG_Y);
                continue;
            }

            let particle_left_position = left(particle_position);
            let particle_below_left_position = below_left(particle_position);
            let can_fall_left_down = {
                is_eligible_particle(particle_below_left_position)
                    && is_eligible_particle(particle_left_position)
            };

            let particle_right_position = right(particle_position);
            let particle_below_right_position = below_right(particle_position);
            let can_fall_right_down = {
                is_eligible_particle(particle_below_right_position)
                    && is_eligible_particle(particle_right_position)
            };

            let other_particle_position = if can_fall_left_down && can_fall_right_down {
                let choice = grid.center_chunk_mut().rng().gen_range(0..2);
                if choice == 0 {
                    particle_left_position
                } else {
                    particle_right_position
                }
            } else if can_fall_left_down {
                particle_left_position
            } else if can_fall_right_down {
                particle_right_position
            } else {
                continue;
            };

            grid.swap_particles(particle_position, other_particle_position);
            grid.center_chunk_mut()
                .attributes_mut()
                .velocity
                .set(particle.id, other_particle_position - particle_position);
        }
    }
}

fn can_fall_into(
    grid: &mut ChunkNeighborhoodView<'_>,
    other_particle_position: IVec2,
    material_states: &MaterialStates,
    particle: crate::particle_grid::Particle,
    material_densities: &MaterialDensities,
) -> bool {
    let other_particle = *grid.get_particle(other_particle_position);
    if other_particle.material == particle.material {
        return false;
    }
    if material_states[other_particle.material] == StateOfMatter::Solid {
        return false;
    }
    if grid.get_dirty(other_particle_position) {
        return false;
    }

    return (material_densities[particle.material] > material_densities[other_particle.material])
        || (material_densities[particle.material] == material_densities[other_particle.material]
            && grid.center_chunk_mut().rng().gen_bool(0.01));
}
