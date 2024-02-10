use bevy::{
    ecs::system::{Query, Res, ResMut},
    utils::HashMap,
};
use rand::{seq::SliceRandom, Rng};

use crate::{
    falling_sand::{FallingSandGrid, FallingSandRng},
    material::{Material, MaterialReactions},
};

pub fn react(
    mut grid_query: Query<&mut FallingSandGrid>,
    material_reactions: Res<MaterialReactions>,
    mut rng: ResMut<FallingSandRng>,
) {
    for mut grid in grid_query.iter_mut() {
        for x in random_dir_range(&mut rng, grid.size().x) {
            for y in random_dir_range(&mut rng, grid.size().y) {
                let particle = *grid.get(x, y).unwrap();
                let particle_is_dirty = *grid.particle_dirty.get(particle.id).unwrap();
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
                            if let Some(adjacent_particle) = grid.get(x + dx, y + dy) {
                                if let Some(reaction) = material_reactions[particle.material]
                                    [adjacent_particle.material]
                                    .as_ref()
                                {
                                    *nearby_materials
                                        .entry(reaction.product_material())
                                        .or_insert(0) += reaction.probability();
                                }
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
                grid.set(x, y, *r.unwrap().0);
            }
        }
    }
}

fn random_dir_range(rng: &mut FallingSandRng, length: i32) -> Box<dyn Iterator<Item = i32>> {
    let reverse = rng.0.gen_bool(0.5);
    if reverse {
        Box::new((0..length).rev())
    } else {
        Box::new(0..length)
    }
}
