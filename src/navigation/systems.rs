use super::components::GoTarget;
use crate::mapping::MAP_RESOLUTION;
use crate::mapping::resources::{Map, MapData, MapPixel};
use bevy::prelude::*;
use either::Either;
use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::blender_editor;

use std::time::Instant;

macro_rules! duration_check {
    ($name:expr, $code:block) => {{
        let start = Instant::now();
        let result = $code;
        println!("'{}' time: {:?}", $name, start.elapsed());
        result
    }};
}

fn to_hash(deger: i32) -> u8 {
    let mut hasher = DefaultHasher::new();
    deger.hash(&mut hasher);
    let hash_sonucu: u64 = hasher.finish();
    hash_sonucu as u8
}

pub fn process_moving_target(
    mut robot: Query<(&mut Transform, &mut GoTarget)>,
    mut gizmos: Gizmos,
    time: Res<Time>,
) {
    'component: for (mut transform, mut go_target) in robot.iter_mut() {
        if let GoTarget::InProgress { path, counter } = go_target.as_mut() {
            if *counter >= path.len() {
                continue;
            }
            for line in path.windows(2) {
                gizmos.line(
                    Vec3::new(line[0].x, 0.3, line[0].y),
                    Vec3::new(line[1].x, 0.3, line[1].y),
                    Color::linear_rgb(1.0, 0.0, 0.0),
                );
            }

            let robot_pos = Vec2::new(transform.translation.x, transform.translation.z);

            let mut target = path[*counter];
            let move_line = time.delta_secs() * 1.0;

            for i in *counter..path.len() {
                let pos = path[i];
                if pos.distance_squared(robot_pos) > move_line * move_line {
                    target = pos;
                    break;
                } else {
                    if i + 1 == path.len() {
                        *go_target = GoTarget::Success;
                        continue 'component;
                    }
                    *counter = i + 1;
                }
            }

            let path_dir = path[*counter - 1] - path[*counter];
            let rotation = Quat::from_rotation_arc(
                Vec3::X,
                Vec3::new(path_dir.y, 0.0, -path_dir.x).normalize(),
            );
            transform.rotation = transform.rotation.slerp(rotation, 5.0 * time.delta_secs());

            transform.translation = transform.translation.move_towards(
                Vec3::new(target.x, transform.translation.y, target.y),
                move_line,
            );
        }
    }
}

pub fn check_go_target_comand(
    mut robot: Query<(Entity, &blender_editor::editor::Robot)>,
    transform_query: Query<&GlobalTransform>,
    map: ResMut<Map>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    for (robot_entity, generator) in robot.iter_mut() {
        if keyboard_input.just_pressed(KeyCode::Space) {
            let Ok(lidar_transform) = transform_query.get(generator.lidar) else {
                continue;
            };

            let origin = lidar_transform.translation();

            let Some(map_data) = &map.map_data else {
                continue;
            };

            if let Some(path) = duration_check!("hesap", {
                map_data.calculate_path(
                    Vec2::new(origin.x, origin.z),
                    Vec2::new(2.0, 2.0),
                    generator.radius,
                )
            }) {
                commands.entity(robot_entity).insert(GoTarget::new(path));
            }
        }
    }
}

impl MapData {
    fn calculate_path(&self, start_pos: Vec2, end_pos: Vec2, radius: f32) -> Option<Vec<Vec2>> {
        const END_FLAG: i32 = -2;
        const OBSTACLE_FLAG: i32 = -1;

        let bounds_check = |pos: (i32, i32)| -> bool {
            pos.0 >= 0 && pos.0 < self.size.0 as i32 && pos.1 >= 0 && pos.1 < self.size.1 as i32
        };

        let start = self.to_pixel(start_pos);
        let end = self.to_pixel(end_pos);

        if !(bounds_check(start) && bounds_check(end)) {
            return None;
        }
        let mut tmp_data = self.mark_obstacles_radius(radius, OBSTACLE_FLAG);

        let idx = |pos: (i32, i32)| -> usize { pos.1 as usize * self.size.0 + pos.0 as usize };

        tmp_data[idx(end)] = END_FLAG;
        tmp_data[idx(start)] = 1;
        let mut points = VecDeque::from_iter([start]);
        'fill_search: loop {
            let Some((x, y)) = points.pop_front() else {
                return None;
            };

            let offsets_list = [(0, 1), (1, 0), (0, -1), (-1, 0)];
            let distance = tmp_data[idx((x, y))];
            for (ox, oy) in offsets_list {
                let pos = (x + ox, y + oy);
                if !bounds_check(pos) {
                    continue;
                }
                let data = tmp_data[idx(pos)];
                if data == 0 {
                    tmp_data[idx(pos)] = distance + 1;
                    points.push_back(pos);
                }
                if data == END_FLAG {
                    tmp_data[idx(pos)] = distance + 1;
                    break 'fill_search;
                }
            }
        }

        let mut path = vec![self.to_world(end)];
        let mut path_cursor = end;
        let mut swap = true;
        for distance in (1..tmp_data[idx(end)]).rev() {
            let offsets_list = [(0, 1), (1, 0), (0, -1), (-1, 0)];

            let iter = if swap {
                Either::Left(offsets_list.iter().rev())
            } else {
                Either::Right(offsets_list.iter())
            };

            for (ox, oy) in iter {
                let pos = (path_cursor.0 + ox, path_cursor.1 + oy);
                if !bounds_check(pos) {
                    continue;
                }

                if tmp_data[idx(pos)] == distance {
                    path_cursor = pos;
                    path.push(self.to_world(pos));
                    break;
                }
            }
            swap = !swap;
        }

        path.reverse();

        // let save_data: Vec<u8> = tmp_data.iter().map(|val| to_hash(*val)).collect();
        // image::save_buffer(
        //     "map_edited.png",
        //     &save_data,
        //     self.size.0 as u32,
        //     self.size.1 as u32,
        //     image::ExtendedColorType::L8,
        // )
        // .unwrap();

        path = Self::smooth_path(path, 0.5);
        for _ in 0..100 {
            path = Self::smooth_path(path, 1.0);
        }

        Some(path)
    }

    fn mark_obstacles_radius(&self, radius: f32, obstacle_flag: i32) -> Vec<i32> {
        let mut tmp_data: Vec<i32> = self
            .data
            .iter()
            .map(|val| match val {
                MapPixel::Free => 0,
                _ => obstacle_flag,
            })
            .collect();
        let pixel_radius = (radius / MAP_RESOLUTION) as i32;

        let check_radius_in = |pos: (i32, i32)| -> bool {
            for x in
                (pos.0 - pixel_radius).max(0)..=(pos.0 + pixel_radius).min(self.size.0 as i32 - 1)
            {
                for y in (pos.1 - pixel_radius).max(0)
                    ..=(pos.1 + pixel_radius).min(self.size.1 as i32 - 1)
                {
                    let check_distance = || -> bool {
                        let diff = (pos.0 - x, pos.1 - y);
                        diff.0.pow(2) + diff.1.pow(2) < pixel_radius.pow(2)
                    };
                    if self.get_pixel((x, y)) != MapPixel::Free && check_distance() {
                        return true;
                    }
                }
            }
            false
        };

        for x in 0..self.size.0 as i32 {
            for y in 0..self.size.1 as i32 {
                if self.get_pixel((x, y)) == MapPixel::Free && check_radius_in((x, y)) {
                    tmp_data[y as usize * self.size.0 + x as usize] = obstacle_flag;
                }
            }
        }
        tmp_data
    }

    fn smooth_path(path: Vec<Vec2>, factor: f32) -> Vec<Vec2> {
        let mut result: Vec<Vec2> = Vec::new();
        if let Some(first) = path.first() {
            result.push(*first);
        }

        for part in path.windows(3) {
            let target = (part[0] + part[2]) * 0.5;
            result.push(part[1].lerp(target, factor));
        }

        if path.len() > 1
            && let Some(last) = path.last()
        {
            result.push(*last);
        }

        result
    }
}
