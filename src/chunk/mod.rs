//! Tiles organised into chunks for efficiency and performance.
//!
//! Mostly everything in this module is private API and not intended to be used
//! outside of this crate as a lot goes on under the hood that can cause issues.
//! With that being said, everything that can be used with helping a chunk get
//! created does live in here.
//!
//! These below examples have nothing to do with this library as all should be
//! done through the [`Tilemap`]. These are just more specific examples which
//! use the private API of this library.
//!
//! [`Tilemap`]: crate::tilemap::Tilemap
//!
//! # Simple chunk creation
//! ```
//! use bevy_asset::{prelude::*, HandleId};
//! use bevy_sprite::prelude::*;
//! use bevy_tilemap::prelude::*;
//!
//! // This must be set in Asset<TextureAtlas>.
//! let texture_atlas_handle = Handle::weak(HandleId::random::<TextureAtlas>());
//!
//! let mut tilemap = Tilemap::new(texture_atlas_handle, 32, 32);
//!
//! // There are two ways to create a new chunk. Either directly...
//!
//! tilemap.insert_chunk((0, 0));
//!
//! // Or indirectly...
//!
//! let point = (0, 0);
//! let sprite_index = 0;
//! let tile = Tile { point, sprite_index, ..Default::default() };
//! tilemap.insert_tile(tile);
//!
//! ```
//!
//! # Specifying what kind of chunk
//! ```
//! use bevy_asset::{prelude::*, HandleId};
//! use bevy_sprite::prelude::*;
//! use bevy_tilemap::prelude::*;
//!
//! // This must be set in Asset<TextureAtlas>.
//! let texture_atlas_handle = Handle::weak(HandleId::random::<TextureAtlas>());
//!
//! let mut tilemap = Tilemap::new(texture_atlas_handle, 32, 32);
//!
//! tilemap.insert_chunk((0, 0));
//!
//! let sprite_order = 0;
//! tilemap.add_layer(TilemapLayer { kind: LayerKind::Dense, ..Default::default() }, 1);
//!
//! let sprite_order = 1;
//! tilemap.add_layer(TilemapLayer { kind: LayerKind::Dense, ..Default::default() }, 1);
//! ```

/// Chunk entity.
pub(crate) mod entity;
/// Sparse and dense chunk layers.
mod layer;
/// Meshes for rendering to vertices.
pub(crate) mod mesh;
/// Raw tile that is stored in the chunks.
pub mod raw_tile;
/// Files and helpers for rendering.
pub(crate) mod render;
/// Systems for chunks.
pub(crate) mod system;

use crate::{lib::*, tile::Tile};
pub use layer::LayerKind;
use layer::{DenseLayer, LayerKindInner, SparseLayer, SpriteLayer};
pub use raw_tile::RawTile;

#[derive(Debug)]
pub(crate) struct Chunk {
    /// The point coordinate of the chunk.
    point: Point2,
    /// The sprite layers of the chunk.
    z_layers: Vec<Vec<SpriteLayer>>,
    /// Ephemeral user data that can be used for flags or other purposes.
    user_data: u128,
    /// A chunks mesh used for rendering.
    mesh: Handle<Mesh>,
    entity: Option<Entity>,
    /// Contains a map of all collision entities.
    #[cfg(feature = "bevy_rapier2d")]
    pub collision_entities: HashMap<usize, Entity>,
}

impl Chunk {
    /// A newly constructed chunk from a point and the maximum number of layers.
    pub(crate) fn new(
        point: Point2,
        layers: &[Option<LayerKind>],
        dimensions: Dimension3,
        mesh: Handle<Mesh>,
    ) -> Chunk {
        let mut chunk = Chunk {
            point,
            z_layers: vec![vec![
                SpriteLayer {
                    inner: LayerKindInner::Sparse(SparseLayer::new(HashMap::default())),
                    entity: None,
                };
                layers.len()
            ]],
            user_data: 0,
            mesh,
            entity: None,
            #[cfg(feature = "bevy_rapier2d")]
            collision_entities: HashMap::default(),
        };

        for (sprite_order, kind) in layers.iter().enumerate() {
            if let Some(kind) = kind {
                chunk.add_layer(kind, sprite_order, dimensions.into())
            }
        }

        chunk
    }

    /// Adds a layer from a layer kind, the z layer, and dimensions of the
    /// chunk.
    pub(crate) fn add_layer(
        &mut self,
        kind: &LayerKind,
        sprite_order: usize,
        dimensions: Dimension3,
    ) {
        for z in 0..dimensions.depth as usize {
            match kind {
                LayerKind::Dense => {
                    let tiles = vec![
                        RawTile {
                            index: 0,
                            color: Color::rgba(0.0, 0.0, 0.0, 0.0)
                        };
                        (dimensions.width * dimensions.height) as usize
                    ];
                    if let Some(z_layer) = self.z_layers.get_mut(z) {
                        if let Some(sprite_order_layer) = z_layer.get_mut(sprite_order) {
                            *sprite_order_layer = SpriteLayer {
                                inner: LayerKindInner::Dense(DenseLayer::new(tiles)),
                                entity: None,
                            };
                        }
                    } else {
                        error!("sprite layer {} is out of bounds", sprite_order);
                    }
                }
                LayerKind::Sparse => {
                    if let Some(z_layer) = self.z_layers.get_mut(z) {
                        if let Some(sprite_order_layer) = z_layer.get_mut(sprite_order) {
                            *sprite_order_layer = SpriteLayer {
                                inner: LayerKindInner::Sparse(SparseLayer::new(HashMap::default())),
                                entity: None,
                            };
                        } else {
                            error!("sprite layer {} is out of bounds", sprite_order);
                        }
                    }
                }
            }
        }
    }

    /// Returns the point of the location of the chunk.
    pub(crate) fn point(&self) -> Point2 {
        self.point
    }

    // /// Returns a copy of the user data.
    // pub(crate) fn user_data(&self) -> u128 {
    //     self.user_data
    // }
    //
    // /// Returns a mutable reference to the user data.
    // pub(crate) fn user_data_mut(&mut self) -> &mut u128 {
    //     &mut self.user_data
    // }

    /// Moves a layer from a z layer to another.
    pub(crate) fn move_sprite_order(&mut self, from_sprite_order: usize, to_sprite_order: usize) {
        for z in 0..self.z_layers.len() {
            if self.z_layers.get(from_sprite_order).is_some() {
                error!(
                    "sprite layer {} exists and can not be moved",
                    to_sprite_order
                );
                return;
            }
            if let Some(sprite_layer) = self.z_layers.get_mut(z) {
                sprite_layer.swap(from_sprite_order, to_sprite_order);
            }
        }
    }

    pub(crate) fn swap_sprite_order(&mut self, from_sprite_order: usize, to_sprite_order: usize) {
        for z in 0..self.z_layers.len() {
            if let Some(sprite_layer) = self.z_layers.get_mut(z) {
                sprite_layer.swap(from_sprite_order, to_sprite_order);
            }
        }
    }

    /// Removes a layer from the specified layer.
    pub(crate) fn remove_layer(&mut self, sprite_order: usize) {
        for _z in 0..self.z_layers.len() {
            self.z_layers.get_mut(sprite_order).take();
        }
    }

    /// Sets the mesh for the chunk layer to use.
    pub(crate) fn set_mesh(&mut self, mesh: Handle<Mesh>) {
        self.mesh = mesh;
    }

    /// Sets a single raw tile to be added to a z layer and index.
    pub(crate) fn set_tile(&mut self, index: usize, tile: Tile<Point3>) {
        if let Some(z_depth) = self.z_layers.get_mut(tile.point.z as usize) {
            if let Some(layer) = z_depth.get_mut(tile.sprite_order) {
                let raw_tile = RawTile {
                    index: tile.sprite_index,
                    color: tile.tint,
                };
                layer.inner.as_mut().set_tile(index, raw_tile);
            } else {
                error!("sprite layer {} does not exist", tile.sprite_order);
            }
        }
    }

    /// Removes a tile from a sprite layer with a given index and z order.
    pub(crate) fn remove_tile(&mut self, index: usize, sprite_order: usize, z_depth: usize) {
        if let Some(z_depth) = self.z_layers.get_mut(z_depth) {
            if let Some(layer) = z_depth.get_mut(sprite_order) {
                layer.inner.as_mut().remove_tile(index);
            } else {
                error!("can not remove tile on sprite layer {}", sprite_order);
            }
        } else {
            error!("sprite layer {} does not exist", sprite_order);
        }
    }

    /// Adds an entity to a z layer, always when it is spawned.
    pub(crate) fn add_entity(&mut self, entity: Entity) {
        self.entity = Some(entity);
    }

    /// Adds an entity to a tile index in a layer.
    #[cfg(feature = "bevy_rapier2d")]
    pub(crate) fn insert_collision_entity(
        &mut self,
        index: usize,
        entity: Entity,
    ) -> Option<Entity> {
        self.collision_entities.insert(index, entity)
    }

    /// Gets the layers entity, if any. Useful for despawning.
    pub(crate) fn take_entity(&mut self) -> Option<Entity> {
        self.entity.take()
    }

    /// Gets the collision entity if any.
    #[cfg(feature = "bevy_rapier2d")]
    pub(crate) fn get_collision_entity(&self, index: usize) -> Option<Entity> {
        self.collision_entities.get(&index).cloned()
    }

    // /// Gets all the layers entities for use with bulk despawning.
    // pub(crate) fn get_entities(&self) -> Vec<Entity> {
    //     let mut entities = Vec::new();
    //     for sprite_layer in &self.z_layers {
    //         if let Some(layer) = sprite_layer {
    //             if let Some(entity) = layer.entity {
    //                 entities.push(entity);
    //             }
    //         }
    //     }
    //     entities
    // }

    /// Gets a reference to a tile from a provided z order and index.
    pub(crate) fn get_tile(
        &self,
        index: usize,
        sprite_order: usize,
        z_depth: usize,
    ) -> Option<&RawTile> {
        self.z_layers.get(z_depth).and_then(|z_depth| {
            z_depth
                .get(sprite_order)
                .and_then(|layer| layer.inner.as_ref().get_tile(index))
        })
    }

    /// Gets a mutable reference to a tile from a provided z order and index.
    pub(crate) fn get_tile_mut(
        &mut self,
        index: usize,
        sprite_order: usize,
        z_depth: usize,
    ) -> Option<&mut RawTile> {
        self.z_layers.get_mut(z_depth).and_then(|z_depth| {
            z_depth
                .get_mut(sprite_order)
                .and_then(|layer| layer.inner.as_mut().get_tile_mut(index))
        })
    }

    /// Gets a vec of all the tiles in the layer, if any.
    #[cfg(feature = "bevy_rapier2d")]
    pub(crate) fn get_tile_indices(&self, sprite_order: usize) -> Option<Vec<usize>> {
        self.z_layers.get(sprite_order).and_then(|layer| {
            layer
                .as_ref()
                .map(|layer| layer.inner.as_ref().get_tile_indices())
        })
    }

    /// At the given z layer, changes the tiles into attributes for use with
    /// the renderer using the given dimensions.
    ///
    /// Easier to pass in the dimensions opposed to storing it everywhere.
    pub(crate) fn tiles_to_renderer_parts(
        &self,
        dimensions: Dimension3,
    ) -> (Vec<f32>, Vec<[f32; 4]>) {
        let area = dimensions.area() as usize;
        let mut tile_indices = Vec::new();
        let mut tile_colors = Vec::new();
        for depth in self.z_layers {
            for layer in depth {
                let (mut indices, mut colors) = layer.inner.as_ref().tiles_to_attributes(area);
                tile_indices.append(&mut indices);
                tile_colors.append(&mut colors);
            }
        }
        (tile_indices, tile_colors)
    }
}
