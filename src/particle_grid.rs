use bevy::reflect::Reflect;

use bytemuck::NoUninit;
use ndarray::prelude::*;

use crate::material::Material;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Reflect, NoUninit)]
#[repr(C)]
pub struct ParticleId(u32);

#[derive(Debug, Clone, Copy, NoUninit, Reflect)]
#[repr(C)]
pub struct Particle {
    pub material: Material,
    pub id: ParticleId,
}

impl Particle {
    pub fn new(material: Material, id: u32) -> Particle {
        Particle {
            material,
            id: ParticleId(id),
        }
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
    pub fn new(size: (usize, usize)) -> ParticleGrid {
        ParticleGrid(Array2::from_shape_fn(size, |(i, j)| {
            let id = j as u32 * size.0 as u32 + i as u32;
            Particle::new(Material::Air, id)
        }))
    }
}

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

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut()
    }
}
