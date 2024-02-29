use rand::Rng;

use bevy::{ecs::system::Res, log::info_span, math::IVec2};

use crate::{
    chunk_neighborhood_view::ChunkNeighborhoodView,
    material::{MaterialDensities, MaterialFlowing, MaterialStates, StateOfMatter},
    process_chunks::{process_chunks_neighborhood, ChunksParam},
    util::{below, left, random_dir_range, right},
};
pub fn flow(
    grid: ChunksParam,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
    material_flowing: Res<MaterialFlowing>,
) {
    process_chunks_neighborhood(&grid, |_chunk_pos, grid| {
        flow_chunk(
            grid,
            &material_flowing,
            &material_densities,
            &material_states,
        )
    });
}

pub fn flow_chunk(
    grid: &mut ChunkNeighborhoodView,
    material_flowing: &MaterialFlowing,
    material_densities: &MaterialDensities,
    material_states: &MaterialStates,
) {
    let span = info_span!("flow_chunk");
    let _guard = span.enter();
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

            if particle.dirty() || !material_flowing[particle.material()] {
                continue;
            }

            let particle_neighorhood_position = particle_chunk_position + chunk_size;
            // Don't flow on top of a less dense material
            let particle_below_position = below(particle_neighorhood_position);
            if material_densities[grid.get_particle(particle_below_position).material()]
                < material_densities[particle.material()]
            {
                continue;
            }

            let mut can_flow_into = |other_particle_position| {
                can_flow_into(
                    grid,
                    other_particle_position,
                    material_states,
                    particle,
                    material_densities,
                )
            };

            let particle_neighorhood_position = particle_chunk_position + chunk_size;
            let particle_left_position = left(particle_neighorhood_position);
            let particle_right_position = right(particle_neighorhood_position);
            let can_flow_left = can_flow_into(particle_left_position);
            let can_flow_right = can_flow_into(particle_right_position);

            let other_particle_position = if can_flow_left && can_flow_right {
                let x_velocity = grid
                    .center_chunk_mut()
                    .attributes()
                    .velocity
                    .get(particle.id())
                    .unwrap()
                    .x;
                if x_velocity == 0 {
                    match grid.center_chunk_mut().rng().gen_range(0..2) {
                        0 => particle_left_position,
                        1 => particle_right_position,
                        _ => unreachable!(),
                    }
                } else {
                    match x_velocity {
                        -1 => particle_left_position,
                        1 => particle_right_position,
                        _ => unreachable!(),
                    }
                }
            } else if can_flow_left {
                particle_left_position
            } else if can_flow_right {
                particle_right_position
            } else {
                continue;
            };

            grid.swap_particles(particle_neighorhood_position, other_particle_position);
            grid.center_chunk_mut().attributes_mut().velocity.set(
                particle.id(),
                other_particle_position - particle_neighorhood_position,
            );
        }
    }
}

fn can_flow_into(
    grid: &mut ChunkNeighborhoodView<'_>,
    other_particle_position: IVec2,
    material_states: &MaterialStates,
    particle: crate::particle_grid::Particle,
    material_densities: &MaterialDensities,
) -> bool {
    let other_particle = *grid.get_particle(other_particle_position);
    if other_particle.material() == particle.material() || other_particle.dirty() {
        return false;
    }

    if material_states[other_particle.material()] == StateOfMatter::Solid {
        return false;
    }
    return (material_densities[particle.material()]
        > material_densities[other_particle.material()])
        || (material_densities[particle.material()]
            == material_densities[other_particle.material()]
            && grid.center_chunk_mut().rng().gen_bool(0.01));
}
