use bevy::prelude::*;

#[derive(Component)]
pub enum GoTarget {
    InProgress { path: Vec<Vec2>, counter: usize },
    Success,
    Failed,
}

impl GoTarget {
    pub fn new(path: Vec<Vec2>) -> Self {
        GoTarget::InProgress {
            path: path,
            counter: 1,
        }
    }
}
