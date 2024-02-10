use rand::Rng;

use bevy::ecs::system::{Query, Res, ResMut};

use crate::{
    falling_sand::{FallingSandGrid, FallingSandRng},
    types::{MaterialDensities, MaterialFlowing, MaterialStates, StateOfMatter},
};

pub fn fall(
    mut grid_query: Query<&mut FallingSandGrid>,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
    mut rng: ResMut<FallingSandRng>,
) {
    for mut grid in grid_query.iter_mut() {
        for x in 0..grid.size().x {
            for y in 0..grid.size().y {
                let particle = grid.get(x, y).unwrap();
                if material_states[particle.material] != StateOfMatter::Liquid
                    || *grid.particle_dirty.get(particle.id).unwrap()
                {
                    continue;
                }

                let can_fall_down = {
                    if y == 0 {
                        false
                    } else {
                        let particle_below = grid.get(x, y - 1).unwrap();
                        material_states[particle_below.material] != StateOfMatter::Solid
                            && particle_below.material != particle.material
                            && material_densities[particle.material]
                                > material_densities[particle_below.material]
                    }
                };
                if can_fall_down {
                    grid.swap_particles((x, y), (x, y - 1));
                    continue;
                }

                let can_fall_left_down = {
                    if x == 0 || y == 0 {
                        false
                    } else {
                        let particle_left_below = grid.get(x - 1, y - 1).unwrap();
                        material_states[particle_left_below.material] != StateOfMatter::Solid
                            && particle_left_below.material != particle.material
                            && material_densities[particle.material]
                                > material_densities[particle_left_below.material]
                    }
                };
                let can_fall_right_down = {
                    if x == grid.size().x - 1 || y == 0 {
                        false
                    } else {
                        let particle_right_below = grid.get(x + 1, y - 1).unwrap();
                        material_states[particle_right_below.material] != StateOfMatter::Solid
                            && particle_right_below.material != particle.material
                            && material_densities[particle.material]
                                > material_densities[particle_right_below.material]
                    }
                };

                if can_fall_left_down && can_fall_right_down {
                    let choice = rng.0.gen_range(0..2);
                    if choice == 0 {
                        grid.swap_particles((x, y), (x - 1, y - 1));
                        continue;
                    } else {
                        grid.swap_particles((x, y), (x + 1, y - 1));
                        continue;
                    }
                }

                if can_fall_left_down {
                    grid.swap_particles((x, y), (x - 1, y - 1));
                    continue;
                }
                if can_fall_right_down {
                    grid.swap_particles((x, y), (x + 1, y - 1));
                    continue;
                }
            }
        }
    }
}

pub fn flow(
    mut grid_query: Query<&mut FallingSandGrid>,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
    material_flowing: Res<MaterialFlowing>,
    mut rng: ResMut<FallingSandRng>,
) {
    for mut grid in grid_query.iter_mut() {
        for x in 0..grid.size().x {
            for y in 0..grid.size().y {
                let particle = grid.get(x, y).unwrap();
                let particle_dirty = grid.particle_dirty.get(particle.id).unwrap();
                if material_states[particle.material] != StateOfMatter::Liquid
                    || !material_flowing[particle.material]
                    || *particle_dirty
                {
                    continue;
                }

                let can_flow_left = {
                    if x == 0 {
                        false
                    } else {
                        let particle_left = grid.get(x - 1, y).unwrap();
                        material_states[particle_left.material] == StateOfMatter::Gas
                            && particle_left.material != particle.material
                            && material_densities[particle.material]
                                > material_densities[particle_left.material]
                    }
                };
                let can_flow_right = {
                    if x == grid.size().x - 1 {
                        false
                    } else {
                        let particle_right = grid.get(x + 1, y).unwrap();
                        material_states[particle_right.material] == StateOfMatter::Gas
                            && particle_right.material != particle.material
                            && material_densities[particle.material]
                                > material_densities[particle_right.material]
                    }
                };

                // If both can flow left and right, choose randomly
                if can_flow_left && can_flow_right {
                    let choice = rng.0.gen_range(0..2);
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
    }
}
