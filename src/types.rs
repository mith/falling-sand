use bevy::prelude::*;

use bytemuck::NoUninit;
use enum_map::EnumMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum, NoUninit)]
#[repr(u8)]
pub enum Material {
    Air = 0,
    Bedrock = 1,
    Sand = 2,
    Water = 3,
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
pub enum Phase {
    Solid,
    Liquid,
    Gas,
}

#[derive(Resource)]
pub struct ToolState {
    pub draw_type: Material,
}

#[derive(Resource)]
pub struct MaterialDensities(pub EnumMap<Material, u32>);

#[derive(Resource)]
pub struct MaterialPhases(pub EnumMap<Material, Phase>);
