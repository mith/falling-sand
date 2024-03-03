use crate::material::Material;

pub const SHIFT: i32 = 6; // log2(CHUNK_SIZE)
pub const CHUNK_SIZE: i32 = 1 << SHIFT;
pub const INITIAL_MATERIAL: Material = Material::Air;
