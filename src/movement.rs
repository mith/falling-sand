use rand::Rng;

use bevy::ecs::system::{Query, Res, ResMut};

use crate::{
    falling_sand::{FallingSandGrid, FallingSandRng},
    material::{Material, MaterialDensities, MaterialFlowing, MaterialStates, StateOfMatter},
};

pub fn fall(
    mut grid_query: Query<&mut FallingSandGrid>,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
    mut rng: ResMut<FallingSandRng>,
) {
    for mut grid in grid_query.iter_mut() {
        for y in 0..grid.size().y {
            let x_iter = x_iter(&mut rng, &grid);
            for x in x_iter {
                let particle = grid.get(x, y).unwrap();
                let particle_is_dirty = grid.particle_dirty.get(particle.id).unwrap();
                if material_states[particle.material] == StateOfMatter::Solid || *particle_is_dirty
                {
                    continue;
                }

                let can_fall_down = {
                    if y == 0 {
                        false
                    } else {
                        let particle_below = grid.get(x, y - 1).unwrap();
                        material_states[particle_below.material] != StateOfMatter::Solid
                            && !*grid.particle_dirty.get(particle_below.id).unwrap()
                            && particle_below.material != particle.material
                            && material_densities[particle.material]
                                > material_densities[particle_below.material]
                    }
                };
                if can_fall_down {
                    grid.swap_particles((x, y), (x, y - 1));
                    continue;
                }

                if particle.material == Material::Air {
                    continue;
                }

                let can_fall_left_down = {
                    if x == 0 || y == 0 {
                        false
                    } else {
                        let particle_left_below = grid.get(x - 1, y - 1).unwrap();
                        material_states[particle_left_below.material] != StateOfMatter::Solid
                            && !*grid.particle_dirty.get(particle_left_below.id).unwrap()
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
                            && !*grid.particle_dirty.get(particle_right_below.id).unwrap()
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
        for y in 0..grid.size().y {
            let x_iter = x_iter(&mut rng, &grid);

            for x in x_iter {
                let particle = grid.get(x, y).unwrap();
                let particle_is_dirty = grid.particle_dirty.get(particle.id).unwrap();
                if !material_flowing[particle.material] || *particle_is_dirty {
                    continue;
                }

                let can_flow_left = {
                    if x == 0 {
                        false
                    } else {
                        let particle_left = grid.get(x - 1, y).unwrap();
                        material_states[particle_left.material] == StateOfMatter::Gas
                            && !*grid.particle_dirty.get(particle_left.id).unwrap()
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
                            && !*grid.particle_dirty.get(particle_right.id).unwrap()
                            && particle_right.material != particle.material
                            && material_densities[particle.material]
                                > material_densities[particle_right.material]
                    }
                };

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

fn x_iter(
    rng: &mut ResMut<'_, FallingSandRng>,
    grid: &bevy::prelude::Mut<'_, FallingSandGrid>,
) -> Box<dyn Iterator<Item = i32>> {
    let reverse_x = rng.0.gen_bool(0.5);
    // 50% chance to reverse the iteration order of x
    let x_iter: Box<dyn Iterator<Item = i32>> = if reverse_x {
        Box::new((0..grid.size().x).rev())
    } else {
        Box::new(0..grid.size().x)
    };
    x_iter
}
