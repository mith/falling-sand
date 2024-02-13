use bevy::{
    ecs::system::{Res, ResMut},
    utils::HashMap,
};
use rand::seq::SliceRandom;

use crate::{
    falling_sand::FallingSandRng,
    falling_sand_grid::FallingSandGridQuery,
    material::{Material, MaterialReactions},
    util::random_dir_range,
};

pub fn react(
    mut grid: FallingSandGridQuery,
    material_reactions: Res<MaterialReactions>,
    mut rng: ResMut<FallingSandRng>,
) {
    for chunk_pos in grid.active_chunks() {
        let chunk_size = grid.chunk_size();
        let min_y = chunk_pos.y * chunk_size.y;
        let max_y = (chunk_pos.y + 1) * chunk_size.y;
        for y in min_y..max_y {
            let min_x = chunk_pos.x * chunk_size.x;
            let max_x = (chunk_pos.x + 1) * chunk_size.x;
            for x in random_dir_range(&mut rng.0, min_x, max_x) {
                let particle = grid.get_particle(x, y);
                let particle_is_dirty: bool = grid.get_dirty(x, y);
                if particle_is_dirty {
                    continue;
                }

                let probable_reactions: HashMap<Material, u32> = {
                    let mut nearby_materials = HashMap::default();
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let adjacent_particle = grid.get_particle(x + dx, y + dy);
                            if grid.get_dirty(x + dx, y + dy) {
                                continue;
                            }
                            if let Some(reaction) = material_reactions
                                .get(particle.material, adjacent_particle.material)
                                .as_ref()
                            {
                                *nearby_materials
                                    .entry(reaction.product_material())
                                    .or_insert(0) += reaction.probability();
                            }
                        }
                    }
                    nearby_materials
                };

                let total_probability: u32 = probable_reactions.values().sum();

                if total_probability == 0 {
                    continue;
                }

                let change_for_no_reaction = 100 - total_probability.min(100);

                let r_vec: Vec<(&Material, &u32)> = probable_reactions
                    .iter()
                    .chain(std::iter::once((
                        &particle.material,
                        &change_for_no_reaction,
                    )))
                    .collect();

                let r = r_vec.choose_weighted(&mut rng.0, |(_, probability)| *probability);
                grid.set_particle(x, y, *r.unwrap().0);
            }
        }
    }
}
