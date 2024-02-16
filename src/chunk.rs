use std::sync::{Arc, RwLock};

use bevy::{
    ecs::component::Component,
    math::IVec2,
    prelude::{Deref, DerefMut},
};
use rand::rngs::StdRng;

use crate::{
    falling_sand_grid::ParticleAttributes,
    material::Material,
    particle_grid::{Particle, ParticleGrid},
};

#[derive(Component, Deref, DerefMut)]
pub struct Chunk(pub Arc<RwLock<ChunkData>>);

impl Chunk {
    pub fn new(size: (usize, usize), rng: StdRng) -> Chunk {
        Chunk(Arc::new(RwLock::new(ChunkData::new(size, rng))))
    }

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
    fn new(size: (usize, usize), rng: StdRng) -> ChunkData {
        ChunkData::new_with_material(size, Material::Air, rng)
    }

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
        let a_id = self
            .particles
            .array()
            .get((a.0 as usize, a.1 as usize))
            .unwrap()
            .id;
        let b_id = self
            .particles
            .array()
            .get((b.0 as usize, b.1 as usize))
            .unwrap()
            .id;

        self.attributes.dirty.set(a_id, true);
        self.attributes.dirty.set(b_id, true);

        // Swap the particles
        self.particles
            .array_mut()
            .swap((a.0 as usize, a.1 as usize), (b.0 as usize, b.1 as usize));

        self.dirty = true;
    }

    pub fn get_particle(&self, x: i32, y: i32) -> Option<&Particle> {
        self.particles.array().get((x as usize, y as usize))
    }

    pub fn get_particle_mut(&mut self, x: i32, y: i32) -> Option<&mut Particle> {
        self.dirty = true;
        self.particles.array_mut().get_mut((x as usize, y as usize))
    }

    pub fn set_particle_material(&mut self, x: i32, y: i32, material: Material) {
        self.dirty = true;
        let particle = self.get_particle_mut(x, y).unwrap();
        particle.material = material;
        let particle_id = particle.id;
        // Mark the particle as dirty
        self.attributes.dirty.set(particle_id, true);
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
