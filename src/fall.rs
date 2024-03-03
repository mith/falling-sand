use rand::Rng;

use bevy::{ecs::system::Res, log::info_span, math::IVec2};

use crate::{
    chunk_neighborhood_view::ChunkNeighborhoodView,
    material::{MaterialDensities, MaterialStates, StateOfMatter},
    particle_grid::Particle,
    process_chunks::{process_chunks_neighborhood, ChunksParam},
    util::{below, below_left, below_right, left, random_dir_range, right},
};

pub fn fall(
    grid: ChunksParam,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
) {
    process_chunks_neighborhood(&grid, |_chunk_pos, grid| {
        fall_chunk(grid, &material_states, &material_densities)
    });
}

pub fn fall_chunk(
    grid: &mut ChunkNeighborhoodView,
    material_states: &MaterialStates,
    material_densities: &MaterialDensities,
) {
    let span = info_span!("fall_chunk");
    let _guard = span.enter();
    const MOMEMTUM_GAIN: u16 = 4096;
    let chunk_size = grid.chunk_size();
    let min_y = 0;
    let max_y = chunk_size.y;
    for y in min_y..max_y {
        let min_x = 0;
        let max_x = chunk_size.x;
        let random_dir_range = {
            let rng = grid.center_chunk_mut().rng();
            random_dir_range(rng, min_x, max_x)
        };
        for x in random_dir_range {
            let particle_chunk_position = IVec2::new(x, y);
            let particle = *grid
                .center_chunk_mut()
                .get_particle(particle_chunk_position)
                .unwrap();
            if particle.dirty() || material_states[particle.material()] == StateOfMatter::Solid {
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

            let particle_neighborhood_position = particle_chunk_position + chunk_size;
            let particle_below_position = below(particle_neighborhood_position);
            if is_eligible_particle(particle_below_position) {
                grid.center_chunk_mut()
                    .attributes_mut()
                    .velocity
                    .set(particle.id(), IVec2::NEG_Y);
                grid.center_chunk_mut()
                    .attributes_mut()
                    .momentum
                    .set(particle.id(), MOMEMTUM_GAIN);
                grid.swap_particles(particle_neighborhood_position, particle_below_position);
                continue;
            }

            let particle_left_position = left(particle_neighborhood_position);
            let particle_below_left_position = below_left(particle_neighborhood_position);
            let can_fall_left_down = {
                is_eligible_particle(particle_below_left_position)
                    && is_eligible_particle(particle_left_position)
            };

            let particle_right_position = right(particle_neighborhood_position);
            let particle_below_right_position = below_right(particle_neighborhood_position);
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

            grid.center_chunk_mut().attributes_mut().velocity.set(
                particle.id(),
                other_particle_position - particle_neighborhood_position,
            );
            grid.center_chunk_mut()
                .attributes_mut()
                .momentum
                .set(particle.id(), MOMEMTUM_GAIN);
            grid.swap_particles(particle_neighborhood_position, other_particle_position);
        }
    }
}

fn can_fall_into(
    grid: &mut ChunkNeighborhoodView,
    other_particle_position: IVec2,
    material_states: &MaterialStates,
    particle: Particle,
    material_densities: &MaterialDensities,
) -> bool {
    let other_particle = *grid.get_particle(other_particle_position);
    if other_particle.dirty()
        || other_particle.material() == particle.material()
        || material_states[other_particle.material()] == StateOfMatter::Solid
    {
        return false;
    }

    (material_densities[particle.material()] > material_densities[other_particle.material()])
        || (material_densities[particle.material()]
            == material_densities[other_particle.material()]
            && grid.center_chunk_mut().rng().gen_bool(0.01))
}
