use bevy::math::IVec2;

use crate::{chunk::ChunkData, particle_grid::ParticleAttributeStore};

macro_rules! define_attributes_and_swap {
    ($($attr:ident: $type:ty),* $(,)?) => {
        #[derive(Debug)]
        pub struct ParticleAttributes {
            $(pub $attr: ParticleAttributeStore<$type>,)*
        }

        impl ParticleAttributes {
            pub fn new(size: usize) -> Self {
                ParticleAttributes {
                    $($attr: ParticleAttributeStore::new(size),)*
                }
            }
        }

        pub fn swap_particles_between_chunks(
            chunk_a: &mut ChunkData,
            particle_pos_a: IVec2,
            chunk_b: &mut ChunkData,
            particle_pos_b: IVec2,
        ) {
            let particle_a = *chunk_a.get_particle(particle_pos_a).unwrap();
            let particle_b = *chunk_b.get_particle(particle_pos_b).unwrap();


            chunk_a.get_particle_mut(particle_pos_a).unwrap().set_material(particle_b.material());
            chunk_b.get_particle_mut(particle_pos_b).unwrap().set_material(particle_a.material());

            $(
                std::mem::swap(
                    chunk_a.attributes_mut().$attr.get_mut(particle_a.id()).unwrap(),
                    chunk_b.attributes_mut().$attr.get_mut(particle_b.id()).unwrap(),
                );
            )*
        }
    };
}

define_attributes_and_swap! {
    velocity: IVec2,
}
