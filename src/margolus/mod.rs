pub mod gravity;

use bevy::prelude::*;
use ndarray::{s, ArrayView2, ArrayViewMut2, Zip};

use crate::{falling_sand::FallingSandGrid, types::Particle};

#[derive(Resource, Default)]
pub struct MargulosState {
    pub odd_timestep: bool,
}

#[derive(Clone, Reflect)]
pub enum BorderUpdateMode {
    CopyEntireSource,
    CopyBorder,
}

#[derive(Resource, Reflect)]
pub struct MargolusSettings {
    pub border_update_mode: BorderUpdateMode,
    pub parallel: bool,
}

impl Default for MargolusSettings {
    fn default() -> Self {
        Self {
            border_update_mode: BorderUpdateMode::CopyBorder,
            parallel: true,
        }
    }
}

pub fn margulos_timestep(mut margulos_state: ResMut<MargulosState>) {
    margulos_state.odd_timestep = !margulos_state.odd_timestep;
}

// TODO: try version without double buffering

pub fn margulos(
    margulos_state: &MargulosState,
    margulos_settings: &MargolusSettings,
    grid: &mut FallingSandGrid,
    function: impl Fn(ArrayViewMut2<Particle>, ArrayView2<Particle>) + Send + Sync,
) {
    let (source, target) = {
        if margulos_state.odd_timestep {
            let (source, target) = grid.0.source_and_target_mut();

            // Copy the border from the source to the target first
            match &margulos_settings.border_update_mode {
                BorderUpdateMode::CopyEntireSource => {
                    target.assign(source);
                }
                BorderUpdateMode::CopyBorder => {
                    target.slice_mut(s![0, ..]).assign(&source.slice(s![0, ..]));
                    target
                        .slice_mut(s![-1, ..])
                        .assign(&source.slice(s![-1, ..]));
                    target.slice_mut(s![.., 0]).assign(&source.slice(s![.., 0]));
                    target
                        .slice_mut(s![.., -1])
                        .assign(&source.slice(s![.., -1]));
                }
            };
            (
                source.slice(s![1..-1, 1..-1]),
                target.slice_mut(s![1..-1, 1..-1]),
            )
        } else {
            let (source, target) = grid.0.source_and_target_mut();
            (source.view(), target.view_mut())
        }
    };

    const CHUNK_SIZE: (usize, usize) = (2, 2);
    let reversed_target = &mut target.reversed_axes();
    let reversed_source = &source.reversed_axes();
    let zipped = Zip::from(reversed_target.exact_chunks_mut(CHUNK_SIZE))
        .and(reversed_source.exact_chunks(CHUNK_SIZE));
    if margulos_settings.parallel {
        zipped.par_for_each(function);
    } else {
        zipped.for_each(function);
    }
    grid.0.swap();
}
