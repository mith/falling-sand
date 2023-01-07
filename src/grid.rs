use bevy::prelude::{Component, Deref, DerefMut};
use bevy::reflect::TypeUuid;

use ndarray::prelude::*;

use crate::double_buffered::DoubleBuffered;
use crate::types::Material;

#[derive(Debug, TypeUuid, Clone, Deref, DerefMut)]
#[uuid = "3e6c203c-76a0-4acc-a812-8d48ee685e61"]
pub struct Grid(pub Array2<Material>);

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        Grid(Array2::from_elem((width, height), Material::Air))
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct FallingSandGrid(pub DoubleBuffered<Grid>);

impl FallingSandGrid {
    pub fn new_from_grid(grid: Grid) -> FallingSandGrid {
        FallingSandGrid(DoubleBuffered::new(grid.clone(), grid.clone()))
    }

    pub fn size(&self) -> (usize, usize) {
        self.0.source().dim()
    }
}
