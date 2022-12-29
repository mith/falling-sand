use bevy::prelude::*;

use enum_map::EnumMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Material {
    Bedrock,
    Air,
    Sand,
    Water,
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
