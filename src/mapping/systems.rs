use avian3d::prelude::*;
use bevy::prelude::*;

use super::resources::*;
use super::{MAP_CHUNK_SIZE, MAP_RESOLUTION};
use crate::blender_editor;

pub fn map_generator(
    spatial_query: SpatialQuery,
    mut robot: Query<&blender_editor::editor::Robot>,
    transform_query: Query<&GlobalTransform>,
    // mut gizmos: Gizmos,
    mut map: ResMut<Map>,
    mut acc: Local<f32>,
    mut delay: Local<f32>,
    time: Res<Time>,
) {
    if *delay < 1.0 {
        *delay += time.delta_secs();
        return;
    }

    const SCAN_HZ: f32 = 10.0;

    *acc += time.delta_secs();
    if *acc >= 1.0 / SCAN_HZ {
        *acc -= 1.0 / SCAN_HZ;
    } else {
        return;
    }

    for generator in robot.iter_mut() {
        if !generator.slam_active {
            continue;
        }
        let Ok(lidar_transform) = transform_query.get(generator.lidar) else {
            continue;
        };

        let origin = lidar_transform.translation();

        let map_data = map.map_data.get_or_insert_with(|| MapData {
            data: vec![MapPixel::Undefined; MAP_CHUNK_SIZE * MAP_CHUNK_SIZE * 4],
            size: (MAP_CHUNK_SIZE * 2, MAP_CHUNK_SIZE * 2),
            origin_point: (MAP_CHUNK_SIZE as i32, MAP_CHUNK_SIZE as i32),
            map_pos: Vec2::new(origin.x, origin.z),
        });

        let mut lidar_segmants: Vec<LidarSegmant> = Vec::new();
        let angle_step = (std::f32::consts::PI * 2.0) / generator.ray_count as f32;

        for i in 0..generator.ray_count {
            let angle = i as f32 * angle_step;
            let local_dir = {
                let dir = Vec2::from_angle(angle);
                Vec3::new(dir.x, 0.0, dir.y)
            };

            let direction = lidar_transform.rotation() * local_dir;
            let filter =
                SpatialQueryFilter::from_mask(blender_editor::editor::GameLayer::default());
            if let Some(hit) =
                spatial_query.cast_ray(origin, Dir3::new_unchecked(direction), 10.0, true, &filter)
            {
                let hit_point = origin + direction * hit.distance;
                lidar_segmants.push(LidarSegmant {
                    end: Vec2::new(hit_point.x, hit_point.z),
                    is_hit: true,
                });

                // gizmos.line(origin, hit_point, Color::linear_rgb(0.0, 1.0, 0.0));
                // gizmos.circle(hit_point, 0.1, Color::linear_rgb(1.0, 0.0, 0.0));
            } else {
                let end_point = origin + direction * 10.0;
                lidar_segmants.push(LidarSegmant {
                    end: Vec2::new(end_point.x, end_point.z),
                    is_hit: false,
                });

                // gizmos.line(origin, end_point, Color::srgba(0.0, 1.0, 1.0, 0.3));
            }
        }

        map_data.set_lidar_segmants(Vec2::new(origin.x, origin.z), lidar_segmants);

        // let image_pixels: Vec<u8> = map_data
        //     .data
        //     .iter()
        //     .map(|val| match val {
        //         MapPixel::Undefined => 205,
        //         MapPixel::Free => 254,
        //         MapPixel::Obstacle => 0,
        //         _ => 205,
        //     })
        //     .collect();
        //
        // image::save_buffer(
        //     "map.png",
        //     &image_pixels,
        //     map_data.size.0 as u32,
        //     map_data.size.1 as u32,
        //     image::ExtendedColorType::L8,
        // )
        // .unwrap();

        // lz4_flex
    }
}

struct LidarSegmant {
    end: Vec2,
    is_hit: bool,
}

impl MapData {
    fn set_lidar_segmants(&mut self, lidar_start_point: Vec2, lidar_segmants: Vec<LidarSegmant>) {
        if lidar_segmants.is_empty() {
            return;
        }

        let pixel_bounding_box = {
            let (lidar_min, lidar_max) =
                Self::calculate_bounding_box(&lidar_start_point, &lidar_segmants);
            (self.to_pixel(lidar_min), self.to_pixel(lidar_max))
        };

        let calculate_chunk_size = |intersection: i32| -> usize {
            if intersection < 0 {
                return 0;
            }
            (intersection as usize / MAP_CHUNK_SIZE + 1) * MAP_CHUNK_SIZE
        };
        self.expand_the_map(
            calculate_chunk_size(-pixel_bounding_box.0.1),
            calculate_chunk_size(pixel_bounding_box.1.1 - self.size.1 as i32),
            calculate_chunk_size(-pixel_bounding_box.0.0),
            calculate_chunk_size(pixel_bounding_box.1.0 - self.size.0 as i32),
        );

        let start_pixel = self.to_pixel(lidar_start_point);
        for segmant in lidar_segmants {
            let end_pixel = self.to_pixel(segmant.end);
            self.set_line(start_pixel, end_pixel, MapPixel::Free);
            if segmant.is_hit {
                self.set_pixel(end_pixel, MapPixel::Obstacle);
            }
        }
    }

    fn set_line(&mut self, a: (i32, i32), b: (i32, i32), color: MapPixel) {
        let dx = (b.0 - a.0).abs();
        let dy = -(b.1 - a.1).abs();
        let sx = if a.0 < b.0 { 1 } else { -1 };
        let sy = if a.1 < b.1 { 1 } else { -1 };

        let mut err = dx + dy; // dx - dy yerine hata terimi (dx + (-dy))
        let mut x = a.0;
        let mut y = a.1;

        loop {
            if self.get_pixel((x, y)) != MapPixel::Obstacle {
                self.set_pixel((x, y), color);
            }

            if x == b.0 && y == b.1 {
                break;
            }

            let e2 = 2 * err;

            if e2 >= dy {
                err += dy;
                x += sx;
            }

            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn to_pixel(&self, pos: Vec2) -> (i32, i32) {
        let local_pos = pos - self.map_pos;
        (
            (local_pos.x / MAP_RESOLUTION) as i32 + self.origin_point.0,
            (local_pos.y / MAP_RESOLUTION) as i32 + self.origin_point.1,
        )
    }

    pub fn to_world(&self, pixel: (i32, i32)) -> Vec2 {
        let x = (pixel.0 - self.origin_point.0) as f32 * MAP_RESOLUTION + self.map_pos.x;
        let y = (pixel.1 - self.origin_point.1) as f32 * MAP_RESOLUTION + self.map_pos.y;

        Vec2::new(x, y)
    }

    fn set_pixel(&mut self, pos: (i32, i32), color: MapPixel) {
        self.data[pos.1 as usize * self.size.0 + pos.0 as usize] = color;
    }

    pub fn get_pixel(&self, pos: (i32, i32)) -> MapPixel {
        self.data[pos.1 as usize * self.size.0 + pos.0 as usize]
    }

    fn calculate_bounding_box(
        lidar_start_point: &Vec2,
        lidar_segmants: &Vec<LidarSegmant>,
    ) -> (Vec2, Vec2) {
        let mut min = *lidar_start_point;
        let mut max = *lidar_start_point;

        for segment in lidar_segmants {
            let p = segment.end;
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
        }

        (min, max)
    }

    fn expand_the_map(&mut self, up: usize, down: usize, left: usize, right: usize) {
        let horizontal_size = self.size.0 + right + left;
        let vertical_size = self.size.1 + up + down;

        let mut data = vec![MapPixel::Undefined; horizontal_size * vertical_size];

        for i in 0..self.size.1 {
            let src_offset = i * self.size.0;
            let dest_offset = (i + up) * horizontal_size + left;

            data[dest_offset..dest_offset + self.size.0]
                .copy_from_slice(&self.data[src_offset..src_offset + self.size.0]);
        }

        self.data = data;
        self.size.0 = horizontal_size;
        self.size.1 = vertical_size;
        self.origin_point.0 += left as i32;
        self.origin_point.1 += up as i32;
    }
}
