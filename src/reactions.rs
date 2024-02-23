use bevy::ecs::system::{Res, ResMut};
use rand::seq::SliceRandom;
use smallvec::SmallVec;

use crate::{
    material::{Material, MaterialReactions},
    process_chunks::{process_chunks_parallel, ChunksParam},
    util::random_dir_range,
};

type ReactionChoices = SmallVec<[(Material, u32); 8]>;

pub fn react(mut grid: ChunksParam, material_reactions: Res<MaterialReactions>) {
    process_chunks_parallel(&mut grid, |chunk_pos, grid| {
        let chunk_size = grid.chunk_size();
        let min_y = chunk_pos.y * chunk_size.y;
        let max_y = (chunk_pos.y + 1) * chunk_size.y;
        for y in min_y..max_y {
            let min_x = chunk_pos.x * chunk_size.x;
            let max_x = (chunk_pos.x + 1) * chunk_size.x;
            for x in random_dir_range(grid.center_chunk_mut().rng(), min_x, max_x) {
                let particle_position = (x, y).into();
                let particle = grid.get_particle(particle_position);
                let particle_is_dirty: bool = grid.get_dirty(particle_position);
                if particle_is_dirty || !material_reactions.has_reactions_for(particle.material) {
                    continue;
                }

                let mut probable_reactions: ReactionChoices = SmallVec::new();

                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let adjecant_particle_position = (x + dx, y + dy).into();
                        let adjacent_particle = grid.get_particle(adjecant_particle_position);
                        if grid.get_dirty(adjecant_particle_position) {
                            continue;
                        }
                        if let Some(reaction) =
                            material_reactions.get(particle.material, adjacent_particle.material)
                        {
                            // Add the probability of the reaction to the existing reaction if it exists
                            // or create a new reaction with the probability of the reaction
                            let reaction_probability = reaction.probability();
                            let reaction_material = reaction.product_material();

                            let existing_reaction = probable_reactions
                                .iter_mut()
                                .find(|(m, _)| *m == reaction_material);
                            if let Some((_, prob)) = existing_reaction {
                                *prob += reaction_probability;
                            } else {
                                probable_reactions.push((reaction_material, reaction_probability));
                            }
                        }
                    }
                }

                let total_probability: u32 = probable_reactions.iter().map(|&(_, prob)| prob).sum();
                let change_for_no_reaction = 100u32.saturating_sub(total_probability);
                probable_reactions.push((particle.material, change_for_no_reaction));

                let total_probability: u32 = probable_reactions
                    .iter()
                    .fold(0, |acc, &(_, prob)| acc + prob);

                if total_probability == 0 {
                    continue;
                }

                let change_for_no_reaction = 100 - total_probability.min(100);

                probable_reactions.push((particle.material, change_for_no_reaction));

                let r = probable_reactions
                    .choose_weighted(grid.center_chunk_mut().rng(), |(_, probability)| {
                        *probability
                    });
                grid.set_particle(particle_position, r.unwrap().0);
            }
        }
    });
}
