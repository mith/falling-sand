use std::sync::{Arc, RwLock};

use bevy::{
    ecs::component::Component,
    math::IVec2,
    prelude::{Deref, DerefMut},
};
use rand::rngs::StdRng;

use crate::{
    material::Material,
    particle_attributes::ParticleAttributes,
    particle_grid::{Particle, ParticleGrid},
};

#[derive(Component, Deref, DerefMut, Clone)]
pub struct Chunk(pub Arc<RwLock<ChunkData>>);

impl Chunk {
    pub fn new_with_material(size: (usize, usize), material: Material, rng: StdRng) -> Chunk {
        Chunk(Arc::new(RwLock::new(ChunkData::new_with_material(
            size, material, rng,
        ))))
    }
}

#[derive(Debug)]
pub struct ChunkData {
    particles: ParticleGrid,
    attributes: ParticleAttributes,
    rng: StdRng,
    dirty: bool,
}

impl ChunkData {
    fn new_with_material(size: (usize, usize), material: Material, rng: StdRng) -> ChunkData {
        let particle_grid = ParticleGrid::new(size, material);
        let size = particle_grid.array().len();
        ChunkData {
            particles: particle_grid,
            attributes: ParticleAttributes::new(size),
            rng,
            dirty: false,
        }
    }

    pub fn particles(&self) -> &ParticleGrid {
        &self.particles
    }

    pub fn particles_mut(&mut self) -> &mut ParticleGrid {
        &mut self.particles
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

    pub fn swap_particles(&mut self, a: IVec2, b: IVec2) {
        self.get_particle_mut(a).unwrap().set_dirty(true);
        self.get_particle_mut(b).unwrap().set_dirty(true);

        // Swap the particles
        self.particles
            .array_mut()
            .swap((a.x as usize, a.y as usize), (b.x as usize, b.y as usize));

        self.dirty = true;
    }

    pub fn get_particle(&self, IVec2 { x, y }: IVec2) -> Option<&Particle> {
        self.particles.array().get((x as usize, y as usize))
    }

    pub fn get_particle_mut(&mut self, IVec2 { x, y }: IVec2) -> Option<&mut Particle> {
        self.dirty = true;
        self.particles.array_mut().get_mut((x as usize, y as usize))
    }

    pub fn set_particle_material(&mut self, position: IVec2, material: Material) {
        self.dirty = true;
        let particle = self.get_particle_mut(position).unwrap();
        particle.set_material(material);
        particle.set_dirty(true);
    }

    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }
}
