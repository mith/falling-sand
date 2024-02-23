use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

use bevy::{
    ecs::system::{Res, ResMut},
    math::IVec2,
};

use crate::{
    chunk,
    falling_sand::FallingSandRng,
    material::{MaterialDensities, MaterialFlowing, MaterialStates, StateOfMatter},
    process_chunks::{process_chunks, process_chunks_parallel, ChunksParam},
    util::{below, below_left, below_right, left, random_dir_range, right},
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
                let particle_position = IVec2::new(x, y);
                let particle = *grid.get_particle(particle_position);
                let particle_is_dirty: bool = grid.get_dirty(particle_position);
                if material_states[particle.material] == StateOfMatter::Solid || particle_is_dirty {
                    continue;
                }

                let mut is_eligible_particle = |other_particle_position| {
                    let other_particle = grid.get_particle(other_particle_position);
                    let not_solid =
                        material_states[other_particle.material] != StateOfMatter::Solid;
                    let not_dirty = !grid.get_dirty(other_particle_position);
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
    });
}

enum FlowDirection {
    Left,
    Right,
}

impl Distribution<FlowDirection> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FlowDirection {
        match rng.gen_range(0..2) {
            0 => FlowDirection::Left,
            1 => FlowDirection::Right,
            _ => unreachable!(),
        }
    }
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
                let particle_position = IVec2::new(x, y);
                let particle = *grid.get_particle(particle_position);
                let particle_is_dirty: bool = grid.get_dirty(particle_position);
                if !material_flowing[particle.material] || particle_is_dirty {
                    continue;
                }

                // Don't flow on top of a less dense material
                let particle_below_position = below(particle_position);
                if material_densities[grid.get_particle(particle_below_position).material]
                    < material_densities[particle.material]
                {
                    continue;
                }

                let mut can_flow_into = |other_particle_position| {
                    let other_particle = *grid.get_particle(other_particle_position);
                    let not_solid =
                        material_states[other_particle.material] != StateOfMatter::Solid;
                    let not_dirty = !grid.get_dirty(other_particle_position);
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

                let particle_left_position = left(particle_position);
                let particle_right_position = right(particle_position);
                let can_flow_left = can_flow_into(particle_left_position);
                let can_flow_right = can_flow_into(particle_right_position);

                let flow_direction = if can_flow_left && can_flow_right {
                    let x_velocity = grid.get_velocity(particle_position).x;
                    if x_velocity == 0 {
                        grid.center_chunk_mut().rng().gen()
                    } else {
                        match x_velocity {
                            -1 => FlowDirection::Left,
                            1 => FlowDirection::Right,
                            _ => unreachable!(),
                        }
                    }
                } else if can_flow_left {
                    FlowDirection::Left
                } else if can_flow_right {
                    FlowDirection::Right
                } else {
                    continue;
                };

                let other_particle_position = match flow_direction {
                    FlowDirection::Left => particle_left_position,
                    FlowDirection::Right => particle_right_position,
                };
                grid.swap_particles(particle_position, other_particle_position);
                grid.center_chunk_mut()
                    .attributes_mut()
                    .velocity
                    .set(particle.id, other_particle_position - particle_position);
            }
        }
    });
}
