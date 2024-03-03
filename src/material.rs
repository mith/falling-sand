use std::fmt;

use bevy::{prelude::*, utils::HashMap};

use bytemuck::{Contiguous, NoUninit};
use enum_map::EnumMap;

use crate::consts::INITIAL_MATERIAL;

pub struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn build(&self, app: &mut App) {
        let material_color: MaterialColor = default();
        app.insert_resource(ClearColor(
            material_color[INITIAL_MATERIAL].as_rgba_linear(),
        ));
        app.insert_resource(material_color)
            .init_resource::<MaterialDensities>()
            .init_resource::<MaterialStates>()
            .init_resource::<MaterialFlowing>()
            .init_resource::<MaterialReactions>();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum, NoUninit, Reflect, Hash)]
#[repr(u16)]
pub enum Material {
    Air = 0,
    Bedrock = 1,
    Sand = 2,
    Water = 3,
    Fire = 4,
    Smoke = 5,
    Wood = 6,
    Steam = 7,
    Oil = 8,
}

impl fmt::Display for Material {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Material::Air => write!(f, "Air"),
            Material::Bedrock => write!(f, "Bedrock"),
            Material::Sand => write!(f, "Sand"),
            Material::Water => write!(f, "Water"),
            Material::Fire => write!(f, "Fire"),
            Material::Smoke => write!(f, "Smoke"),
            Material::Wood => write!(f, "Wood"),
            Material::Steam => write!(f, "Steam"),
            Material::Oil => write!(f, "Oil"),
        }
    }
}

impl From<Material> for u16 {
    fn from(material: Material) -> u16 {
        unsafe { std::mem::transmute(material) }
    }
}

impl From<u16> for Material {
    fn from(value: u16) -> Material {
        unsafe { std::mem::transmute(value) }
    }
}

unsafe impl Contiguous for Material {
    type Int = u32;

    const MIN_VALUE: Self::Int = 0;
    const MAX_VALUE: Self::Int = 8;
}

impl TryFrom<u32> for Material {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Air),
            1 => Ok(Self::Bedrock),
            2 => Ok(Self::Sand),
            3 => Ok(Self::Water),
            4 => Ok(Self::Fire),
            5 => Ok(Self::Smoke),
            6 => Ok(Self::Wood),
            7 => Ok(Self::Steam),
            8 => Ok(Self::Oil),
            _ => Err(()),
        }
    }
}

pub struct MaterialIterator {
    next_value: u32,
}

impl MaterialIterator {
    pub fn new() -> Self {
        MaterialIterator { next_value: 0 }
    }
}

impl Iterator for MaterialIterator {
    type Item = Material;

    fn next(&mut self) -> Option<Self::Item> {
        let material = Material::try_from(self.next_value).ok();
        self.next_value += 1;
        material
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum StateOfMatter {
    Solid,
    Liquid,
    Gas,
}

#[derive(Resource, Deref)]
pub struct MaterialDensities(pub EnumMap<Material, u32>);

impl Default for MaterialDensities {
    fn default() -> Self {
        MaterialDensities(enum_map! {
            Material::Air => 10,
            Material::Fire => 1,
            Material::Smoke => 1,
            Material::Water => 1000,
            Material::Sand => 1600,
            Material::Bedrock => 3000,
            Material::Wood => 500,
            Material::Steam => 1,
            Material::Oil => 800,
        })
    }
}

#[derive(Resource, Deref)]
pub struct MaterialStates(pub EnumMap<Material, StateOfMatter>);

impl Default for MaterialStates {
    fn default() -> Self {
        MaterialStates(enum_map! {
            Material::Air => StateOfMatter::Gas,
            Material::Fire => StateOfMatter::Gas,
            Material::Smoke => StateOfMatter::Gas,
            Material::Water => StateOfMatter::Liquid,
            Material::Sand => StateOfMatter::Liquid,
            Material::Bedrock => StateOfMatter::Solid,
            Material::Wood => StateOfMatter::Solid,
            Material::Steam => StateOfMatter::Gas,
            Material::Oil => StateOfMatter::Liquid,
        })
    }
}

#[derive(Resource, Deref)]
pub struct MaterialFlowing(pub EnumMap<Material, bool>);

impl Default for MaterialFlowing {
    fn default() -> Self {
        MaterialFlowing(enum_map! {
            Material::Air => true,
            Material::Bedrock => false,
            Material::Sand => false,
            Material::Water => true,
            Material::Fire => true,
            Material::Smoke => true,
            Material::Wood => false,
            Material::Steam => true,
            Material::Oil => true,
        })
    }
}

#[derive(Resource, Deref)]
pub struct MaterialColor(pub EnumMap<Material, Color>);

impl Default for MaterialColor {
    fn default() -> Self {
        MaterialColor(enum_map! {
            Material::Air => Color::rgb_u8(240, 248, 255u8),
            Material::Bedrock => Color::rgb_u8(50, 50, 50u8),
            Material::Sand => Color::rgb_u8(194, 178, 128u8),
            Material::Water => Color::rgb_u8(28, 107, 160u8),
            Material::Fire => Color::rgb_u8(255, 165, 0u8),
            Material::Smoke => Color::rgb_u8(160, 160, 160u8),
            Material::Wood => Color::rgb_u8(160, 82, 45u8),
            Material::Steam => Color::rgb_u8(230, 230, 230u8),
            Material::Oil => Color::rgb_u8(40, 40, 0u8),
        })
    }
}

pub struct Reaction {
    /// The probability of the reaction happening, from 0 to 100
    probability: u32,
    /// The resulting material of the reaction
    product_material: Material,
}

impl Reaction {
    pub fn probability(&self) -> u32 {
        self.probability
    }

    pub fn product_material(&self) -> Material {
        self.product_material
    }
}

#[derive(Resource, Deref)]
pub struct MaterialReactions(HashMap<(Material, Material), Reaction>);

impl MaterialReactions {
    pub fn get(&self, material: Material, adjacent_material: Material) -> Option<&Reaction> {
        self.0.get(&(material, adjacent_material))
    }

    pub fn has_reactions_for(&self, material: Material) -> bool {
        self.0.keys().any(|(m1, _m2)| *m1 == material)
    }
}

impl Default for MaterialReactions {
    fn default() -> Self {
        MaterialReactions(HashMap::from([
            (
                (Material::Water, Material::Fire),
                Reaction {
                    probability: 1000,
                    product_material: Material::Steam,
                },
            ),
            (
                (Material::Wood, Material::Fire),
                Reaction {
                    probability: 1500,
                    product_material: Material::Fire,
                },
            ),
            (
                (Material::Oil, Material::Fire),
                Reaction {
                    probability: 4000,
                    product_material: Material::Fire,
                },
            ),
            (
                (Material::Smoke, Material::Air),
                Reaction {
                    probability: 5,
                    product_material: Material::Air,
                },
            ),
        ]))
    }
}
