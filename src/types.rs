use bevy::prelude::*;

use bytemuck::{Contiguous, NoUninit};
use enum_map::EnumMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum, NoUninit, Reflect)]
#[repr(u32)]
pub enum Material {
    Air = 0,
    Bedrock = 1,
    Sand = 2,
    Water = 3,
}

unsafe impl Contiguous for Material {
    type Int = u32;

    const MIN_VALUE: Self::Int = 0;
    const MAX_VALUE: Self::Int = 3;
}

impl From<u8> for Material {
    fn from(value: u8) -> Self {
        match value {
            0 => Material::Air,
            1 => Material::Bedrock,
            2 => Material::Sand,
            3 => Material::Water,
            _ => panic!("Invalid material"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum StateOfMatter {
    Solid,
    Liquid,
    Gas,
}

#[derive(Resource)]
pub struct ToolState {
    pub draw_type: Material,
}

#[derive(Resource, Deref)]
pub struct MaterialDensities(pub EnumMap<Material, u32>);

#[derive(Resource, Deref)]
pub struct MaterialStates(pub EnumMap<Material, StateOfMatter>);

#[derive(Default, Clone)]
pub struct ParticleVelocity(pub Vec2);
