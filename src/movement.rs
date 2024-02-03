use std::ops::DerefMut;

use bevy::{
    ecs::system::{Query, Res},
    math::{IVec2, Quat, Vec2, Vec3Swizzles},
    time::{Time, Virtual},
    transform::components::Transform,
};
use line_drawing::Bresenham;

use crate::{
    falling_sand::FallingSandGrid,
    particle_grid::Particle,
    types::{MaterialDensities, MaterialStates, StateOfMatter},
};

pub fn gravity(
    mut grid_query: Query<&mut FallingSandGrid>,
    material_states: Res<MaterialStates>,
    time: Res<Time<Virtual>>,
) {
    for mut grid in grid_query.iter_mut() {
        let FallingSandGrid {
            ref mut particles,
            particle_velocities,
            ..
        } = grid.deref_mut();
        for ((x, y), particle) in particles.grid().indexed_iter() {
            if material_states[particle.material] != StateOfMatter::Liquid {
                continue;
            }

            let can_fall_down = {
                if y == 0 {
                    false
                } else {
                    let particle_below = particles.grid()[(x, y - 1)];
                    material_states[particle_below.material] != StateOfMatter::Solid
                        || particle_below.material != particle.material
                }
            };

            let can_fall_left_down = {
                if x == 0 || y == 0 {
                    false
                } else {
                    let particle_left_below = particles.grid()[(x - 1, y - 1)];
                    material_states[particle_left_below.material] != StateOfMatter::Solid
                        || particle_left_below.material != particle.material
                }
            };

            let can_fall_right_down = {
                if x == particles.grid().dim().0 - 1 || y == 0 {
                    false
                } else {
                    let particle_right_below = particles.grid()[(x + 1, y - 1)];
                    material_states[particle_right_below.material] != StateOfMatter::Solid
                        || particle_right_below.material != particle.material
                }
            };

            if !(can_fall_down || can_fall_left_down || can_fall_right_down) {
                continue;
            }

            let particle_velocity = particle_velocities.get(particle.id).unwrap().0.y;

            particle_velocities.get_mut(particle.id).unwrap().0.y =
                (particle_velocity - 9.8 * time.delta_seconds()).max(-20.);
        }
    }
}

pub fn move_particles(
    mut grid_query: Query<&mut FallingSandGrid>,
    material_densities: Res<MaterialDensities>,
    material_states: Res<MaterialStates>,
    time: Res<Time<Virtual>>,
) {
    for mut grid in grid_query.iter_mut() {
        let dim = grid.particles.grid().dim();
        let FallingSandGrid {
            particles,
            particle_velocities,
            particle_dirty,
            particle_positions,
        } = grid.deref_mut();
        for x in 0..dim.0 {
            for y in 0..dim.1 {
                let i = (x as i32, y as i32);

                let particle = particles.grid()[(x, y)];
                if *particle_dirty.get(particle.id).unwrap() {
                    continue;
                }
                let particle_velocity =
                    particle_velocities.get(particle.id).unwrap().0 * time.delta_seconds();
                if particle_velocity == Vec2::ZERO {
                    continue;
                }

                let particle_density = material_densities[particle.material];

                let particle_position = *particle_positions.get(particle.id).unwrap();

                let new_position = particle_position + particle_velocity;

                let new_i = (
                    new_position.x.clamp(0., dim.0 as f32).round() as i32,
                    new_position.y.clamp(0., dim.1 as f32).round() as i32,
                );

                if new_i == i {
                    *particle_positions.get_mut(particle.id).unwrap() = new_position;
                    continue;
                }

                let mut old_position = particle_position;
                for (other_x, other_y) in Bresenham::new(i, new_i).skip(1) {
                    let v_i = (other_x as usize, other_y as usize);
                    let other_particle = particles.grid()[v_i];
                    let other_particle_dirty = *particle_dirty.get(other_particle.id).unwrap();
                    let other_particle_density = material_densities[other_particle.material];
                    if other_particle_dirty {
                        break;
                    }

                    let mut new_position = Vec2::new(other_x as f32, other_y as f32);

                    if material_states[other_particle.material] == StateOfMatter::Solid
                        || other_particle.material == particle.material
                        || particle_density < other_particle_density
                    {
                        // Deflect the particle by sliding past the other particle diagonally or
                        // along the surface

                        // If the velocity is rotated by 45 degrees and the new position is in a empty cell
                        // and the cell at a 90 degree angle is also empty, then move the particle

                        let (rot45_particle, rot45_position) = rotated_other_particle(
                            std::f32::consts::FRAC_PI_4,
                            particle_velocity,
                            old_position,
                            dim,
                            particles,
                        );

                        let (rot90_particle, _) = rotated_other_particle(
                            std::f32::consts::FRAC_PI_2,
                            particle_velocity,
                            old_position,
                            dim,
                            particles,
                        );

                        if !*particle_dirty.get(rot45_particle.id).unwrap()
                            && !*particle_dirty.get(rot90_particle.id).unwrap()
                            && material_states[rot45_particle.material] != StateOfMatter::Solid
                            && material_states[rot90_particle.material] != StateOfMatter::Solid
                            && material_densities[rot45_particle.material] < particle_density
                            && material_densities[rot90_particle.material] < particle_density
                        {
                            particles.grid_mut().swap(
                                (x, y),
                                (rot45_position.x as usize, rot45_position.y as usize),
                            );
                            *particle_positions.get_mut(particle.id).unwrap() =
                                rot45_position.as_vec2();
                            *particle_dirty.get_mut(particle.id).unwrap() = true;
                            *particle_dirty.get_mut(rot45_particle.id).unwrap() = true;
                            *particle_positions.get_mut(rot45_particle.id).unwrap() = old_position;
                        } else {
                            particle_velocities.get_mut(particle.id).unwrap().0 = Vec2::ZERO;
                            break;
                        }
                    }

                    // Swap the particles
                    particles.grid_mut().swap((x, y), v_i);
                    *particle_positions.get_mut(particle.id).unwrap() = new_position;
                    *particle_positions.get_mut(other_particle.id).unwrap() = old_position;
                    *particle_dirty.get_mut(particle.id).unwrap() = true;
                    *particle_dirty.get_mut(other_particle.id).unwrap() = true;

                    old_position = Vec2::new(other_x as f32, other_y as f32);
                }
            }
        }
    }
}

fn rotated_other_particle(
    rotation: f32,
    particle_velocity: Vec2,
    old_position: Vec2,
    dim: (usize, usize),
    particles: &mut crate::particle_grid::ParticleGrid,
) -> (Particle, IVec2) {
    let rotated_velocity = Quat::from_rotation_z(rotation)
        .mul_vec3(particle_velocity.extend(0.))
        .xy();

    let rotated_new_position = old_position.round() + rotated_velocity.normalize();

    let rotated_i = (
        rotated_new_position.x.clamp(0., dim.0 as f32).round() as i32,
        rotated_new_position.y.clamp(0., dim.1 as f32).round() as i32,
    );

    (
        particles.grid()[(rotated_i.0 as usize, rotated_i.1 as usize)],
        IVec2::new(rotated_i.0, rotated_i.1),
    )
}
