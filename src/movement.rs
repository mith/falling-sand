use rand::{rngs::StdRng, Rng};

use bevy::{
    ecs::system::{Query, Res, ResMut},
    math::IVec2,
    utils::HashSet,
};

use crate::{
    chunk::Chunk,
    falling_sand::FallingSandRng,
    falling_sand_grid::FallingSandGridQuery,
    material::{MaterialDensities, MaterialFlowing, MaterialStates, StateOfMatter},
    particle_grid::Particle,
};

pub fn fall_2(
    mut grid: FallingSandGridQuery,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
    mut rng: ResMut<FallingSandRng>,
) {
    for pos in grid.active_chunks() {
        let chunk_size = grid.chunk_size();
        let min_y = pos.y * chunk_size.y;
        let max_y = (pos.y + 1) * chunk_size.y;
        for y in min_y..max_y {
            let min_x = pos.x * chunk_size.x;
            let max_x = (pos.x + 1) * chunk_size.x;
            for x in random_dir_range(&mut rng.0, min_x, max_x) {
                let particle = *grid.get_particle(x, y);
                let particle_is_dirty: bool = grid.get_dirty(x, y);
                if material_states[particle.material] == StateOfMatter::Solid || particle_is_dirty {
                    continue;
                }

                let mut is_eligible_particle = |(x_b, y_b)| {
                    let p = *grid.get_particle(x_b, y_b);
                    material_states[p.material] != StateOfMatter::Solid
                        && !grid.get_dirty(x_b, y_b)
                        && p.material != particle.material
                        && (material_densities[particle.material] > material_densities[p.material]
                            || material_densities[particle.material]
                                == material_densities[p.material]
                                && rng.0.gen_bool(0.01))
                };

                if is_eligible_particle((x, y - 1)) {
                    grid.swap_particles((x, y), (x, y - 1));
                    continue;
                }
            }
        }
    }
}

pub fn fall(
    mut grid_query: Query<&mut Chunk>,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
    mut rng: ResMut<FallingSandRng>,
) {
    for mut grid in grid_query.iter_mut() {
        for y in 1..grid.size().y {
            let x_iter = x_iter(&mut rng, grid.size().x);
            for x in x_iter {
                let particle = grid.get(x, y).unwrap();
                let particle_is_dirty = grid.attributes().dirty.get(particle.id).unwrap();
                if material_states[particle.material] == StateOfMatter::Solid || *particle_is_dirty
                {
                    continue;
                }

                let mut is_eligible_particle = |p: &Particle| {
                    material_states[p.material] != StateOfMatter::Solid
                        && !*grid.attributes().dirty.get(p.id).unwrap()
                        && p.material != particle.material
                        && (material_densities[particle.material] > material_densities[p.material]
                            || material_densities[particle.material]
                                == material_densities[p.material]
                                && rng.0.gen_bool(0.01))
                };

                let particle_below = grid.get(x, y - 1).unwrap();
                if is_eligible_particle(particle_below) {
                    grid.swap_particles((x, y), (x, y - 1));
                    continue;
                }

                if material_densities[particle_below.material]
                    < material_densities[particle.material]
                {
                    continue;
                }

                let can_fall_left_down = {
                    if x == 0 || y == 0 {
                        false
                    } else {
                        let particle_left_below = grid.get(x - 1, y - 1).unwrap();
                        let particle_left = grid.get(x - 1, y).unwrap();
                        is_eligible_particle(particle_left_below)
                            && is_eligible_particle(particle_left)
                    }
                };
                let can_fall_right_down = {
                    if x == grid.size().x - 1 || y == 0 {
                        false
                    } else {
                        let particle_right_below = grid.get(x + 1, y - 1).unwrap();
                        let particle_right = grid.get(x + 1, y).unwrap();
                        is_eligible_particle(particle_right_below)
                            && is_eligible_particle(particle_right)
                    }
                };

                if can_fall_left_down && can_fall_right_down {
                    let choice = rng.0.gen_range(0..2);
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
    }
}

pub fn flow(
    mut grid_query: Query<&mut Chunk>,
    material_states: Res<MaterialStates>,
    material_densities: Res<MaterialDensities>,
    material_flowing: Res<MaterialFlowing>,
    mut rng: ResMut<FallingSandRng>,
) {
    for mut grid in grid_query.iter_mut() {
        for y in 0..grid.size().y {
            let x_iter = x_iter(&mut rng, grid.size().x);

            for x in x_iter {
                let particle = grid.get(x, y).unwrap();
                let particle_is_dirty = grid.attributes().dirty.get(particle.id).unwrap();
                if !material_flowing[particle.material] || *particle_is_dirty {
                    continue;
                }

                let mut can_flow_into = |p: &Particle| {
                    material_states[p.material] != StateOfMatter::Solid
                        && !*grid.attributes().dirty.get(p.id).unwrap()
                        && p.material != particle.material
                        && (material_densities[particle.material] > material_densities[p.material]
                            || material_densities[particle.material]
                                == material_densities[p.material]
                                && rng.0.gen_bool(0.01))
                };

                let can_flow_left = {
                    if x == 0 {
                        false
                    } else {
                        let particle_left = grid.get(x - 1, y).unwrap();
                        can_flow_into(particle_left)
                    }
                };
                let can_flow_right = {
                    if x == grid.size().x - 1 {
                        false
                    } else {
                        let particle_right = grid.get(x + 1, y).unwrap();
                        can_flow_into(particle_right)
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

fn x_iter(rng: &mut ResMut<'_, FallingSandRng>, max: i32) -> Box<dyn Iterator<Item = i32>> {
    let reverse_x = rng.0.gen_bool(0.5);
    // 50% chance to reverse the iteration order of x
    let x_iter: Box<dyn Iterator<Item = i32>> = if reverse_x {
        Box::new((0..max).rev())
    } else {
        Box::new(0..max)
    };
    x_iter
}

fn random_dir_range(rng: &mut StdRng, min: i32, max: i32) -> Box<dyn Iterator<Item = i32>> {
    let reverse = rng.gen_bool(0.5);
    if reverse {
        Box::new((min..max).rev())
    } else {
        Box::new(min..max)
    }
}
