use bevy::{
    ecs::{component::Component, system::Resource},
    math::IVec2,
};
use ndarray::Array2;

#[derive(Component)]
pub struct ChunkPosition(pub IVec2);

#[derive(Resource)]
pub struct SpatialStore<T> {
    pub positions: Array2<Option<T>>,
    pub offset: IVec2,
}

impl<T> Default for SpatialStore<T> {
    fn default() -> Self {
        Self {
            positions: Array2::default((0, 0)),
            offset: IVec2::ZERO,
        }
    }
}

impl<T: Clone> SpatialStore<T> {
    pub fn get_at(&self, position: IVec2) -> Option<&T> {
        self.positions
            .get((
                (position.x + self.offset.x) as usize,
                (position.y + self.offset.y) as usize,
            ))
            .and_then(|x| x.as_ref())
    }

    pub fn contains(&self, position: IVec2) -> bool {
        self.get_at(position).is_some()
    }

    pub fn add(&mut self, position: IVec2, value: T) {
        // Update the bounds and offset if necessary
        let mut new_pos = position + self.offset;
        let (max_x, max_y) = (self.positions.dim().0 as i32, self.positions.dim().1 as i32);

        if new_pos.x >= max_x || new_pos.y >= max_y || new_pos.x < 0 || new_pos.y < 0 {
            self.expand_bounds(position);
            new_pos = position + self.offset;
        }

        self.positions[(new_pos.x as usize, new_pos.y as usize)] = Some(value);
    }

    fn expand_bounds(&mut self, position: IVec2) {
        let min_bounds = position.min(-self.offset);
        let array_dim = self.positions.dim();
        let max_bounds =
            position.max(IVec2::new(array_dim.0 as i32, array_dim.1 as i32) + self.offset);

        let new_offset: IVec2 = (min_bounds.x.min(0).abs(), min_bounds.y.min(0).abs()).into();

        let size: IVec2 = max_bounds + new_offset + IVec2::ONE;

        let mut new_positions = Array2::default((size.x as usize, size.y as usize));

        for y in 0..array_dim.1 {
            for x in 0..array_dim.0 {
                if let Some(value) = &self.positions[(x, y)] {
                    let new_x = x as i32 + new_offset.x - self.offset.x;
                    let new_y = y as i32 + new_offset.y - self.offset.y;
                    new_positions[(new_x as usize, new_y as usize)] = Some(value.clone());
                }
            }
        }

        self.offset = new_offset;
        self.positions = new_positions;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::utils::HashMap;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn spatial_store_state_machine_proptest(operations in prop::collection::vec(any::<Operation>(), 1..100)) {
            let mut store = SpatialStore::<i32>::default();
            let mut reference = HashMap::new();

            for op in operations {
                match op {
                    Operation::Add { position, value } => {
                        store.add(position, value);
                        reference.insert(position, value);
                    },
                    Operation::Get { position } => {
                        let store_result = store.get_at(position);
                        let reference_result = reference.get(&position);
                        prop_assert_eq!(store_result, reference_result);
                    },
                    Operation::Contains { position } => {
                        let store_result = store.contains(position);
                        let reference_result = reference.contains_key(&position);
                        prop_assert_eq!(store_result, reference_result);
                    },
                }
            }
        }
    }

    #[derive(Debug, Clone)]
    enum Operation {
        Add { position: IVec2, value: i32 },
        Get { position: IVec2 },
        Contains { position: IVec2 },
    }

    impl Arbitrary for Operation {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                ((-10..=10i32, -10..=10i32), any::<i32>()).prop_map(|((x, y), value)| {
                    Operation::Add {
                        position: IVec2::new(x, y),
                        value,
                    }
                }),
                (-10..=10i32, -10..=10i32).prop_map(|(x, y)| Operation::Get {
                    position: IVec2::new(x, y)
                }),
                (-10..=10i32, -10..=10i32).prop_map(|(x, y)| Operation::Contains {
                    position: IVec2::new(x, y)
                }),
            ]
            .boxed()
        }
    }
}
