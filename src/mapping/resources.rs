use bevy::prelude::*;

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MapPixel {
    Undefined,
    Free,
    Obstacle,
}

pub struct MapData {
    pub data: Vec<MapPixel>,
    pub size: (usize, usize),
    pub origin_point: (i32, i32),
    pub map_pos: Vec2,
}

#[derive(Resource)]
pub struct Map {
    pub map_data: Option<MapData>,
}

impl Map {
    pub fn new() -> Self {
        Self { map_data: None }
    }
}
