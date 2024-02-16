use rand::Rng;

use bevy::ecs::system::{Res, ResMut};

use crate::{
    chunk,
    falling_sand::FallingSandRng,
    material::{MaterialDensities, MaterialFlowing, MaterialStates, StateOfMatter},
    process_chunks::{process_chunks, process_chunks_parallel, ChunksParam},
    util::random_dir_range,
};

pub fn fall(
    mut grid: ChunksParam,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
) {
    process_chunks_parallel(&mut grid, |chunk_pos, grid| {
        let chunk_size = grid.chunk_size();
        let min_y = chunk_pos.y * chunk_size.y;
        let max_y = (chunk_pos.y + 1) * chunk_size.y;
        for y in min_y..max_y {
            let min_x = chunk_pos.x * chunk_size.x;
            let max_x = (chunk_pos.x + 1) * chunk_size.x;
            let random_dir_range = {
                let rng = grid.center_chunk_mut().rng();
                random_dir_range(rng, min_x, max_x)
            };
            for x in random_dir_range {
                let particle = *grid.get_particle(x, y);
                let particle_is_dirty: bool = grid.get_dirty(x, y);
                if material_states[particle.material] == StateOfMatter::Solid || particle_is_dirty {
                    continue;
                }

                let mut is_eligible_particle = |(x_b, y_b)| {
                    let other_particle = grid.get_particle(x_b, y_b);
                    let not_solid =
                        material_states[other_particle.material] != StateOfMatter::Solid;
                    let not_dirty = !grid.get_dirty(x_b, y_b);
                    let not_same_material = other_particle.material != particle.material;
                    let less_dense = material_densities[particle.material]
                        > material_densities[other_particle.material];
                    let same_density_sometimes = material_densities[particle.material]
                        == material_densities[other_particle.material]
                        && grid.center_chunk_mut().rng().gen_bool(0.01);
                    not_solid
                        && not_dirty
                        && not_same_material
                        && (less_dense || same_density_sometimes)
                };

                if is_eligible_particle((x, y - 1)) {
                    grid.swap_particles((x, y), (x, y - 1));
                    continue;
                }

                let can_fall_left_down =
                    { is_eligible_particle((x - 1, y - 1)) && is_eligible_particle((x - 1, y)) };

                let can_fall_right_down =
                    { is_eligible_particle((x + 1, y - 1)) && is_eligible_particle((x + 1, y)) };

                if can_fall_left_down && can_fall_right_down {
                    let choice = grid.center_chunk_mut().rng().gen_range(0..2);
                    if choice == 0 {
                        grid.swap_particles((x, y), (x - 1, y));
                        grid.swap_particles((x - 1, y), (x - 1, y - 1));
                        continue;
                    } else {
                        grid.swap_particles((x, y), (x + 1, y));
                        grid.swap_particles((x + 1, y), (x + 1, y - 1));
                        continue;
                    }
                }

                if can_fall_left_down {
                    grid.swap_particles((x, y), (x - 1, y));
                    grid.swap_particles((x - 1, y), (x - 1, y - 1));
                    continue;
                }

                if can_fall_right_down {
                    grid.swap_particles((x, y), (x + 1, y));
                    grid.swap_particles((x + 1, y), (x + 1, y - 1));
                    continue;
                }
            }
        }
    });
}

pub fn flow(
    mut grid: ChunksParam,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
    material_flowing: Res<MaterialFlowing>,
) {
    process_chunks_parallel(&mut grid, |chunk_pos, grid| {
        let chunk_size = grid.chunk_size();
        let min_y = chunk_pos.y * chunk_size.y;
        let max_y = (chunk_pos.y + 1) * chunk_size.y;
        for y in min_y..max_y {
            let min_x = chunk_pos.x * chunk_size.x;
            let max_x = (chunk_pos.x + 1) * chunk_size.x;
            let random_dir_range = {
                let rng = grid.center_chunk_mut().rng();
                random_dir_range(rng, min_x, max_x)
            };
            for x in random_dir_range {
                let particle = *grid.get_particle(x, y);
                let particle_is_dirty: bool = grid.get_dirty(x, y);
                if !material_flowing[particle.material] || particle_is_dirty {
                    continue;
                }

                let mut can_flow_into = |(x_b, y_b)| {
                    let other_particle = *grid.get_particle(x_b, y_b);
                    let not_solid =
                        material_states[other_particle.material] != StateOfMatter::Solid;
                    let not_dirty = !grid.get_dirty(x_b, y_b);
                    let not_same_material = other_particle.material != particle.material;
                    let less_dense = material_densities[particle.material]
                        > material_densities[other_particle.material];
                    let same_density_sometimes = material_densities[particle.material]
                        == material_densities[other_particle.material]
                        && grid.center_chunk_mut().rng().gen_bool(0.01);
                    not_solid
                        && not_dirty
                        && not_same_material
                        && (less_dense || same_density_sometimes)
                };

                let can_flow_left = { can_flow_into((x - 1, y)) };
                let can_flow_right = { can_flow_into((x + 1, y)) };

                if can_flow_left && can_flow_right {
                    let choice = grid.center_chunk_mut().rng().gen_range(0..2);
                    if choice == 0 {
                        grid.swap_particles((x, y), (x - 1, y));
                        continue;
                    } else {
                        grid.swap_particles((x, y), (x + 1, y));
                        continue;
                    }
                }

                if can_flow_left {
                    grid.swap_particles((x, y), (x - 1, y));
                    continue;
                }

                if can_flow_right {
                    grid.swap_particles((x, y), (x + 1, y));
                    continue;
                }
            }
        }
    });
}
