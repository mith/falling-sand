use bevy::{ecs::component::Component, math::IVec2};

use crate::{
    falling_sand_grid::ParticleAttributes,
    material::Material,
    particle_grid::{Particle, ParticleGrid},
};

#[derive(Component)]
pub struct Chunk {
    pub particles: ParticleGrid,
    attributes: ParticleAttributes,
}

impl Chunk {
    pub fn new(size: (usize, usize)) -> Chunk {
        let particle_grid = ParticleGrid::new(size);
        let size = particle_grid.array().len();
        Chunk {
            particles: particle_grid,
            attributes: ParticleAttributes::new(size),
        }
    }

    pub fn attributes(&self) -> &ParticleAttributes {
        &self.attributes
    }

    pub fn attributes_mut(&mut self) -> &mut ParticleAttributes {
        &mut self.attributes
    }

    pub fn size(&self) -> IVec2 {
        IVec2::new(
            self.particles.array().dim().0 as i32,
            self.particles.array().dim().1 as i32,
        )
    }

    pub fn swap_particles(&mut self, a: (i32, i32), b: (i32, i32)) {
        // Mark the particles as dirty

        // Swap the particles
        self.particles
            .array_mut()
            .swap((a.0 as usize, a.1 as usize), (b.0 as usize, b.1 as usize));
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&Particle> {
        self.particles.array().get((x as usize, y as usize))
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut Particle> {
        self.particles.array_mut().get_mut((x as usize, y as usize))
    }

    pub fn set(&mut self, x: i32, y: i32, material: Material) {
        let particle = self.get_mut(x, y).unwrap();
        particle.material = material;
        let particle_id = particle.id;
        // Mark the particle as dirty
        self.attributes.dirty.set(particle_id, true);
    }
}
