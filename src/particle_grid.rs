use bevy::reflect::Reflect;
use bitfield::bitfield;
use bytemuck::NoUninit;
use ndarray::prelude::*;

use crate::material::Material;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Reflect, NoUninit)]
#[repr(C)]
pub struct ParticleId(u16);

impl From<ParticleId> for u16 {
    fn from(id: ParticleId) -> u16 {
        id.0
    }
}

impl From<u16> for ParticleId {
    fn from(id: u16) -> ParticleId {
        ParticleId(id)
    }
}

bitfield! {
    #[derive(Clone, Copy, NoUninit, Reflect)]
    #[repr(C)]
    pub struct Particle(u32);
    impl Debug;
    u16;
    pub from into ParticleId, id, set_id: 9, 0;
    pub from into Material, material, set_material: 19, 10;
    pub dirty, set_dirty: 31;
}

impl From<Particle> for u32 {
    fn from(val: Particle) -> Self {
        unsafe { std::mem::transmute(val) }
    }
}

impl Particle {
    pub fn new(material: Material, id: u16) -> Particle {
        let mut particle = Particle(0);
        particle.set_material(material);
        particle.set_id(id.into());
        particle
    }
}

#[derive(Debug, Clone)]
pub struct ParticleGrid(Array2<Particle>);

impl ParticleGrid {
    pub fn array(&self) -> &Array2<Particle> {
        &self.0
    }

    pub fn array_mut(&mut self) -> &mut Array2<Particle> {
        &mut self.0
    }
}

impl ParticleGrid {
    pub fn new(size: (usize, usize), material: Material) -> ParticleGrid {
        ParticleGrid(Array2::from_shape_fn(size, |(i, j)| {
            let id = j as u16 * size.0 as u16 + i as u16;
            Particle::new(material, id)
        }))
    }
}

#[derive(Debug)]
pub struct ParticleAttributeStore<T> {
    data: Vec<T>,
}

impl<T: Default + Clone> ParticleAttributeStore<T> {
    pub fn new(size: usize) -> ParticleAttributeStore<T> {
        ParticleAttributeStore {
            data: vec![T::default(); size],
        }
    }

    pub fn get(&self, id: ParticleId) -> Option<&T> {
        self.data.get(id.0 as usize)
    }

    pub fn get_mut(&mut self, id: ParticleId) -> Option<&mut T> {
        self.data.get_mut(id.0 as usize)
    }

    pub fn set(&mut self, id: ParticleId, value: T) {
        self.data[id.0 as usize] = value;
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut()
    }
}
