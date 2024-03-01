use bevy::math::IVec2;

use crate::{
    chunk::ChunkData,
    particle_grid::{Particle, ParticleAttributeStore},
};

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
                let particle_a: &mut Particle = chunk_a.get_particle_mut(particle_pos_a).unwrap();
                let particle_b: &mut Particle = chunk_b.get_particle_mut(particle_pos_b).unwrap();

                particle_a.set_dirty(true);
                particle_b.set_dirty(true);

                let particle_a_material = particle_a.material();
                let particle_b_material = particle_b.material();

                particle_a.set_material(particle_b_material);
                particle_b.set_material(particle_a_material);

                let particle_a_id = particle_a.id();
                let particle_b_id = particle_b.id();

            $(
                std::mem::swap(
                    chunk_a.attributes_mut().$attr.get_mut(particle_a_id).unwrap(),
                    chunk_b.attributes_mut().$attr.get_mut(particle_b_id).unwrap(),
                );
            )*
        }
    };
}

define_attributes_and_swap! {
    velocity: IVec2,
}
