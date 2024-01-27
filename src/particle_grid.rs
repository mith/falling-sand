use bevy::prelude::{Deref, DerefMut};

use ndarray::prelude::*;

use crate::types::{Material, Particle};

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct ParticleGrid(pub Array2<Particle>);

impl ParticleGrid {
    pub fn new(width: usize, height: usize) -> ParticleGrid {
        ParticleGrid(Array2::from_elem(
            (width, height),
            Particle::new(Material::Air),
        ))
    }
}
