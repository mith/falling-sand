use bevy::prelude::*;
use bevy::reflect::TypeUuid;

use ndarray::prelude::*;
use std::ops::{Deref, DerefMut};

use crate::types::Material;

impl Deref for Board {
    type Target = Array2<Material>;
    fn deref(&self) -> &Array2<Material> {
        &self.0
    }
}

impl DerefMut for Board {
    fn deref_mut(&mut self) -> &mut Array2<Material> {
        &mut self.0
    }
}
#[derive(Debug, TypeUuid, Clone)]
#[uuid = "3e6c203c-76a0-4acc-a812-8d48ee685e61"]
pub struct Board(pub Array2<Material>);

impl Board {
    pub fn new(width: usize, height: usize) -> Board {
        Board(Array2::from_elem((width, height), Material::Air))
    }
}
