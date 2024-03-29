#![allow(clippy::needless_range_loop)]

use audio_engine::{Sound, WavDecoder};
use sprite_render::{Camera, SpriteInstance, SpriteRender, Texture, TextureId};

use winit::{dpi::PhysicalSize, window::WindowId};

use rand::seq::index::sample;
use rand::Rng;

use std::f32::consts::PI;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::time::Instant;

mod utils;

use ezing::*;
fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + (b - a) * t
}

const COLORS: &[[u8; 4]] = &[
    [0, 255, 0, 255],
    [0, 204, 255, 255],
    [102, 0, 255, 255],
    [255, 0, 102, 255],
    [255, 204, 0, 255],
    [0, 255, 76, 255],
    [0, 128, 255, 255],
    [178, 0, 255, 255],
    [255, 0, 26, 255],
    [230, 255, 0, 255],
    [0, 255, 153, 255],
    [0, 51, 255, 255],
    [255, 0, 255, 255],
    [255, 51, 0, 255],
    [153, 255, 0, 255],
    [0, 255, 229, 255],
    [25, 0, 255, 255],
    [255, 0, 179, 255],
    [255, 127, 0, 255],
    [77, 255, 0, 255],
];

mod atlas {
    include!(concat!(env!("OUT_DIR"), "/atlas.rs"));
    pub const PIPES: [[f32; 4]; 5] = [PIPE_ONE, PIPE_TWO_L, PIPE_TWO, PIPE_TREE, PIPE_FOUR];
    pub const NUMBERS: [[f32; 4]; 10] = [N0, N1, N2, N3, N4, N5, N6, N7, N8, N9];
}

mod sounds {
    pub static CLICK: &[u8] = include_bytes!("../res/sound/click.wav");
    pub static CONFIRM: &[u8] = include_bytes!("../res/sound/confirm.wav");
    pub static NEGATE: &[u8] = include_bytes!("../res/sound/negate.wav");
    pub static WHOOSH: &[u8] = include_bytes!("../res/sound/whoosh.wav");
}

struct Pipe {
    dir: u8,
    kind: u8,
    sprite: SpriteInstance,
    angle: f32,
    previous_angle: f32,
    anim_time: f32,
    color_time: f32,
    previous_color: [u8; 4],
    target_color: u16,
}
impl Pipe {
    fn new(x: f32, y: f32, size: f32, texture: TextureId, kind: u8, dir: u8) -> Self {
        Self {
            dir,
            kind,
            sprite: SpriteInstance::new(
                x,
                y,
                size * 1.01,
                size * 1.01,
                texture,
                atlas::PIPES[kind as usize],
            ),
            angle: 0.0,
            previous_angle: 0.0,
            anim_time: 0.000001,
            color_time: 0.000001,
            previous_color: [0; 4],
            target_color: 0,
        }
    }

    fn animate(&mut self, dt: f32) {
        if self.anim_time != 0.0 {
            self.anim_time = (self.anim_time - dt * 0.7).max(0.0);
            let d =
                ((self.dir as f32 * PI / 2.0) - self.previous_angle + PI).rem_euclid(2.0 * PI) - PI;
            let t = elastic_out(1.0 - self.anim_time);
            self.angle = self.previous_angle + lerp(t, 0.0, d);
            self.sprite.set_angle(self.angle);
        }
        if self.color_time != 0.0 {
            self.color_time = (self.color_time - dt * 3.0).max(0.0);
            let t = 1.0 - self.color_time;
            let color = [
                lerp(
                    t,
                    self.previous_color[0] as f32,
                    COLORS[self.target_color as usize % COLORS.len()][0] as f32,
                ) as u8,
                lerp(
                    t,
                    self.previous_color[1] as f32,
                    COLORS[self.target_color as usize % COLORS.len()][1] as f32,
                ) as u8,
                lerp(
                    t,
                    self.previous_color[2] as f32,
                    COLORS[self.target_color as usize % COLORS.len()][2] as f32,
                ) as u8,
                255,
            ];
            self.sprite.set_color(color);
        }
    }

    /// Is called when there is a mouse click over self.
    /// When right is true it is a right mouse button click, otherwise is left.
    fn click(&mut self, right: bool, play_sound: bool) {
        if right {
            self.dir = (self.dir + 1) % 4;
        } else {
            self.dir = (self.dir + 4 - 1) % 4;
        }
        if play_sound {
            crate::audio_engine()
                .new_sound(WavDecoder::new(Cursor::new(sounds::CLICK)).unwrap())
                .unwrap()
                .play();
        }
        self.anim_time = 1.0;
        self.previous_angle = self.angle;
    }

    fn change_color(&mut self, target: u16) {
        if target != self.target_color {
            self.previous_color = self.sprite.color;
            self.target_color = target;
            self.color_time = 1.0;
        }
    }
}

#[derive(Default)]
pub struct Input {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_left_state: u8,
    pub mouse_rigth_state: u8,
}
impl Input {
    /// update button state (from 'Pressed' to 'Down)
    pub fn update(&mut self) {
        self.mouse_left_state = match self.mouse_left_state {
            0 => 0,
            1 => 2,
            3 => 1,
            _ => 0,
        };
        self.mouse_rigth_state = match self.mouse_rigth_state {
            0 => 0,
            1 => 2,
            3 => 1,
            _ => 0,
        };
    }
}

struct GameBoard<R: Rng> {
    sound_effects: bool,
    music: Sound,
    slow_down_effect: Arc<AtomicBool>,
    width: u8,
    height: u8,
    pipes: Vec<Pipe>,
    regions: Vec<u16>,
    region_id_pool: Vec<u16>,
    number_regions: u16,
    color_pool: Vec<u16>,
    number_colors: u16,
    texture: TextureId,
    win_anim: f32,
    lose_anim: f32,
    win_sprite: SpriteInstance,
    highlight_sprite: SpriteInstance,
    again_button: Button,
    level: u32,
    life: u32,
    life_time: f32,
    life_dirty: bool,
    life_text: SpriteInstance,
    life_number: Vec<SpriteInstance>,
    score: u32,
    level_score: u32,
    score_dirty: bool,
    score_text: SpriteInstance,
    score_number: Vec<SpriteInstance>,
    click_count: u32,
    expect_min_click_count: u32,
    game_start: Instant,
    level_start: Instant,
    rng: R,
}
impl<R: Rng> GameBoard<R> {
    pub fn new(
        texture: TextureId,
        rng: R,
        music: Sound,
        slow_down_effect: Arc<AtomicBool>,
    ) -> Self {
        let mut highlight_sprite =
            SpriteInstance::new(-100.0, 0.0, 1.0, 1.0, texture, atlas::BLANCK);
        highlight_sprite.set_color([255, 255, 255, 64]);
        Self {
            sound_effects: true,
            music,
            slow_down_effect,
            width: 0,
            height: 0,
            pipes: Vec::new(),
            regions: Vec::new(),
            region_id_pool: Vec::new(),
            number_regions: 0,
            color_pool: Vec::new(),
            number_colors: 0,
            win_anim: 0.0,
            lose_anim: 0.0,
            win_sprite: SpriteInstance::new(0.0, 0.0, 1.0, 1.0, texture, atlas::YOU_WIN),
            highlight_sprite,
            again_button: Button::new(
                SpriteInstance::new_height_prop(1.2, -0.9, 0.1, texture, atlas::PLAY_AGAIN)
                    .with_color([255, 255, 0, 255]),
                [-0.30, 0.30, -0.06, 0.06],
            ),
            texture,
            level: 0,
            life: 10,
            life_time: 1.0,
            life_dirty: true,
            life_text: SpriteInstance::new_height_prop(1.2, -0.9, 0.1, texture, atlas::TIME)
                .with_color([0, 240, 0, 255]),
            life_number: Vec::new(),
            score: 0,
            level_score: 0,
            score_dirty: true,
            score_text: SpriteInstance::new_height_prop(1.2, -0.9, 0.1, texture, atlas::SCORE)
                .with_color([0, 240, 0, 255]),
            score_number: Vec::new(),
            click_count: 0,
            expect_min_click_count: 0,
            game_start: Instant::now(),
            level_start: Instant::now(),
            rng,
        }
    }

    pub fn reset(&mut self) {
        self.music.play();
        self.win_anim = 0.0;
        self.lose_anim = 0.0;
        self.again_button
            .sprite
            .set_position(-10000000.0, -1000000.0);
        self.level = 0;
        self.life = 0;
        self.life_time = 1.0;
        self.life_dirty = true;
        self.score = 0;
        self.score_dirty = true;
        self.click_count = 0;
        self.game_start = Instant::now();

        if self.life_text.get_x() > 1.0 {
            self.resize(2.0, 1.0);
        } else {
            self.resize(1.0, 2.0);
        }

        self.new_level(4, 4);
    }

    fn new_level(&mut self, width: u8, height: u8) {
        self.width = width;
        self.height = height;
        self.region_id_pool.clear();
        self.number_regions = 0;
        self.color_pool.clear();
        self.number_colors = 0;
        self.pipes = Vec::with_capacity(width as usize * height as usize);
        self.life_time = 1.0;
        let maze = Self::gen_maze(width, height, &mut self.rng);
        let mut i = 0usize;
        let size = 2.0 / self.height as f32;
        self.highlight_sprite.set_size(size * 0.9, size * 0.9);

        self.level_start = Instant::now();
        self.click_count = 0;

        self.level += 1;
        self.level_score = 0;

        let mut total_diff = 0u32;

        for y in 0..height {
            for x in 0..width {
                let (kind, dir) = match maze[i] {
                    0b0001 => (0, 0),
                    0b0010 => (0, 1),
                    0b0100 => (0, 2),
                    0b1000 => (0, 3),

                    0b0011 => (1, 0),
                    0b0110 => (1, 1),
                    0b1100 => (1, 2),
                    0b1001 => (1, 3),

                    0b0101 => (2, 0),
                    0b1010 => (2, 1),

                    0b1110 => (3, 0),
                    0b1101 => (3, 1),
                    0b1011 => (3, 2),
                    0b0111 => (3, 3),

                    0b1111 => (4, 0),
                    _ => (5, 0),
                };

                let diff = self.rng.gen_range(0, 4);
                total_diff += if diff <= 2 { diff } else { 4 - diff };

                self.pipes.push(Pipe::new(
                    (x as f32 - width as f32 / 2.0) * size + size / 2.0,
                    (y as f32 - height as f32 / 2.0) * size + size / 2.0,
                    size,
                    self.texture,
                    kind,
                    (dir + diff as u8) % 4,
                ));
                i += 1;
            }
        }
        self.trace_regions();

        self.expect_min_click_count = total_diff;
        let area = self.width as u32 * self.height as u32;
        let expect_time = 30.0 + 0.307 * area as f32; // + 0.00120 * area as f32 * area as f32;
        let expect_click = 30.0 + 0.542 * area as f32; // + 0.00154 * area as f32 * area as f32;
        self.add_life((expect_time * 2.0 + expect_click) as i32);
    }

    // preference == 0 mean no preference
    fn next_region_id(&mut self, preference: u16) -> u16 {
        if self.region_id_pool.is_empty() {
            self.number_regions += 1;
            self.number_regions
        } else if preference == 0 {
            self.region_id_pool.pop().unwrap()
        } else {
            let mut i = 0;
            loop {
                if i == self.region_id_pool.len() {
                    break self.region_id_pool.pop().unwrap();
                }
                if self.region_id_pool[i] == preference {
                    self.region_id_pool.swap_remove(i);
                    break preference;
                }
                i += 1;
            }
        }
    }

    // If a region is not more valid, it is add back to the pool,
    // and its region size is removed
    fn add_region_to_pool(&mut self, region: u16) {
        if region != 0 && !self.region_id_pool.iter().any(|&x| x == region) {
            self.region_id_pool.push(region);
        }
    }

    fn next_color(&mut self) -> u16 {
        if self.color_pool.is_empty() {
            self.number_colors += 1;
            self.number_colors
        } else {
            self.color_pool.pop().unwrap()
        }
    }

    // If a color is not more used, it is add back to the pool
    fn add_color_to_pool(&mut self, color: u16) {
        if !self.color_pool.iter().any(|&x| x == color) {
            self.color_pool.push(color);
        }
    }

    // Remove a color from to pool to be used
    fn remove_color_from_pool(&mut self, color: u16) {
        match self.color_pool.iter().position(|&x| x == color) {
            Some(i) => {
                self.color_pool.swap_remove(i);
            }
            None => panic!("there is no way that I will remove a color wich is not in the pool"),
        }
    }

    fn trace_region(&mut self, start: i32, region: u16) {
        let neights: [i32; 4] = [1, self.width as i32, -1, -(self.width as i32)];
        let mut explore: Vec<i32> = vec![start];
        self.regions[start as usize] = region;
        let mut visited = vec![false; self.width as usize * self.height as usize];
        visited[start as usize] = true;
        while let Some(curr) = explore.pop() {
            let curr_dir = match self.pipes[curr as usize].kind {
                0 => 0b00010001u8,
                1 => 0b00110011,
                2 => 0b01010101,
                3 => 0b11101110,
                4 => 0b11111111,
                _ => 0,
            }
            .rotate_left(self.pipes[curr as usize].dir as u32);

            for i in 0..4 {
                let next = (curr + neights[i]) as usize;
                if (curr % self.width as i32 - next as i32 % self.width as i32).abs() <= 1
                    && next < self.regions.len()
                    && !visited[next]
                {
                    // if it is inexpored
                    let next_dir = match self.pipes[next as usize].kind {
                        0 => 0b00010001u8,
                        1 => 0b00110011,
                        2 => 0b01010101,
                        3 => 0b11101110,
                        4 => 0b11111111,
                        _ => 0,
                    }
                    .rotate_left(self.pipes[next as usize].dir as u32 + 2);
                    if curr_dir & (1 << i) & next_dir != 0 {
                        explore.push(next as i32);
                        self.regions[next] = region;
                        visited[next] = true;
                    }
                }
            }
        }
    }

    fn trace_regions(&mut self) {
        self.regions = vec![0u16; self.width as usize * self.height as usize];

        // let mut region;
        for i in 0..(self.width as usize * self.height as usize) {
            if self.regions[i] == 0 {
                let region = self.next_region_id(0);
                self.trace_region(i as i32, region);
            }
        }
        for i in 0..(self.width as usize * self.height as usize) {
            self.pipes[i].change_color(self.regions[i] - 1);
        }
        self.number_colors = self.number_regions;
    }

    fn update_regions(&mut self, i: i32) {
        let neights: [i32; 4] = [1, self.width as i32, -1, -(self.width as i32)];

        let mut to_check: Vec<(usize, u16)> = Vec::with_capacity(5);

        for n in &neights {
            let next = (i + n) as usize;
            if (i % self.width as i32 - next as i32 % self.width as i32).abs() <= 1
                && next < self.regions.len()
            {
                to_check.push((next, self.regions[next]));
                self.add_region_to_pool(self.regions[next]);
                self.regions[next] = 0;
            }
        }

        to_check.push((i as usize, self.regions[i as usize]));
        self.add_region_to_pool(self.regions[i as usize]);
        self.regions[i as usize] = 0;

        for (curr, region) in to_check.iter_mut() {
            if self.regions[*curr] == 0 {
                *region = self.next_region_id(*region);
                self.trace_region(*curr as i32, *region);
            }
        }
        let mut colors = Vec::new();
        let mut start = 0;
        let mut used_regions = Vec::new(); //TODO: check used in insertion time
        for (_, region) in to_check.iter() {
            if used_regions.iter().any(|x| *x == *region) {
                continue;
            }
            used_regions.push(*region);
            let mut has_some_region = false;
            for i in 0..(self.width as usize * self.height as usize) {
                if self.regions[i] == *region {
                    has_some_region = true;
                    let color = self.pipes[i].target_color;
                    match colors[start..].iter().position(|(_, x, _)| *x == color) {
                        Some(i) => {
                            colors[start + i].2 += 1;
                        }
                        None => {
                            colors.push((*region, color, 1));
                            self.add_color_to_pool(color);
                        }
                    }
                }
            }
            if !has_some_region {
                used_regions.pop();
            }
            start = colors.len();
        }
        colors.sort_by_key(|x| x.2);
        let mut used_colors = Vec::new();
        while let Some((region, color, _)) = colors.pop() {
            if used_colors.iter().any(|x| *x == color) {
                continue;
            }
            used_colors.push(color);
            match used_regions.iter().position(|x| *x == region) {
                Some(i) => used_regions.swap_remove(i),
                None => continue,
            };
            self.remove_color_from_pool(color);
            for i in 0..(self.width as usize * self.height as usize) {
                if self.regions[i] == region {
                    self.pipes[i].change_color(color);
                }
            }
        }
        for region in used_regions {
            let color = self.next_color();
            for i in 0..(self.width as usize * self.height as usize) {
                if self.regions[i] == region {
                    self.pipes[i].change_color(color);
                }
            }
        }
    }

    fn gen_maze(width: u8, height: u8, rng: &mut R) -> Box<[i8]> {
        let neights: [i32; 4] = [1, width as i32, -1, -(width as i32)];

        let mut grid = vec![0i8; width as usize * height as usize].into_boxed_slice();

        let start = rng.gen_range(0, grid.len());
        let mut path: Vec<i32> = vec![start as i32];
        grid[start] = 0;

        'path: while !path.is_empty() {
            let r = rng.gen_range(0, path.len());
            let curr = path[r];
            for i in sample(rng, 4, 4).iter() {
                let next = (curr + neights[i]) as usize;
                if (curr % width as i32 - next as i32 % width as i32).abs() <= 1
                    && next < grid.len()
                    && grid[next] == 0
                {
                    // if it is inexpored
                    grid[curr as usize] |= 1 << i; // set dir bitmask
                    grid[next] |= 1 << ((i + 2) % 4); // set dir bitmask
                    path.push(next as i32); // add to the backtrack path
                    continue 'path;
                }
            }
            // If there is no neighbor inexpored, backtrack
            path.swap_remove(r);
        }
        // Remove all the dead ends
        for curr in 0..grid.len() as i32 {
            if let 0b0001 | 0b0010 | 0b0100 | 0b1000 = grid[curr as usize] {
                for i in sample(rng, 4, 4).iter() {
                    let next = (curr + neights[i]) as usize;
                    if (curr % width as i32 - next as i32 % width as i32).abs() <= 1
                        && next < grid.len()
                        && grid[next] & (1 << ((i + 2) % 4)) == 0
                    {
                        grid[curr as usize] |= 1 << i; // set dir bitmask
                        grid[next] |= 1 << ((i + 2) % 4); // set dir bitmask
                        break;
                    }
                }
            }
        }

        let mut i = 0;
        while i < width as usize * height as usize {
            match grid[i] {
                0b0001 => print!("╒"),
                0b0010 => print!("╓"),
                0b0100 => print!("╕"),
                0b1000 => print!("╙"),

                0b0011 => print!("╔"),
                0b0110 => print!("╗"),
                0b1100 => print!("╝"),
                0b1001 => print!("╚"),

                0b0101 => print!("═"),
                0b1010 => print!("║"),

                0b1110 => print!("╣"),
                0b1101 => print!("╩"),
                0b1011 => print!("╠"),
                0b0111 => print!("╦"),

                0b1111 => print!("╬"),
                _ => print!(" "),
            };
            i += 1;
            if i % width as usize == 0 {
                println!();
            }
        }
        grid
    }

    fn check_is_done(&self) -> bool {
        let neights: [i32; 2] = [1, self.width as i32];

        // if there is more than one region, it is not done
        if self.number_regions as usize - self.region_id_pool.len() > 1 {
            return false;
        }

        // upper row
        for curr in 0..self.width as i32 {
            let curr_dir = match self.pipes[curr as usize].kind {
                0 => 0b00010001u8,
                1 => 0b00110011,
                2 => 0b01010101,
                3 => 0b11101110,
                4 => 0b11111111,
                _ => 0,
            }
            .rotate_left(self.pipes[curr as usize].dir as u32);

            if curr_dir & (1 << 3) != 0 {
                // if curr is connect to nowhere, it is not done
                return false;
            }
        }

        // left column
        for curr in (0..(self.width as i32 * self.height as i32)).step_by(self.width as usize) {
            let curr_dir = match self.pipes[curr as usize].kind {
                0 => 0b00010001u8,
                1 => 0b00110011,
                2 => 0b01010101,
                3 => 0b11101110,
                4 => 0b11111111,
                _ => 0,
            }
            .rotate_left(self.pipes[curr as usize].dir as u32);

            if curr_dir & (1 << 2) != 0 {
                // if curr is connect to nowhere, it is not done
                return false;
            }
        }

        for curr in 0..(self.width as i32 * self.height as i32) {
            let curr_dir = match self.pipes[curr as usize].kind {
                0 => 0b00010001u8,
                1 => 0b00110011,
                2 => 0b01010101,
                3 => 0b11101110,
                4 => 0b11111111,
                _ => 0,
            }
            .rotate_left(self.pipes[curr as usize].dir as u32);

            for i in 0..2 {
                let next = (curr + neights[i]) as usize;
                if (curr % self.width as i32 - next as i32 % self.width as i32).abs() <= 1
                    && next < self.regions.len()
                {
                    let next_dir = match self.pipes[next as usize].kind {
                        0 => 0b00010001u8,
                        1 => 0b00110011,
                        2 => 0b01010101,
                        3 => 0b11101110,
                        4 => 0b11111111,
                        _ => 0,
                    }
                    .rotate_left(self.pipes[next as usize].dir as u32 + 2);

                    // If curr and next have a unparied connection, it is not done
                    if (curr_dir & (1 << i) != 0) != (next_dir & (1 << i) != 0) {
                        return false;
                    }
                } else if curr_dir & (1 << i) != 0 {
                    // if curr is connect to nowhere, it is not done
                    return false;
                }
            }
        }
        // if there is no pipe with unparied connection, it is done
        true
    }

    fn count_connections(&self) -> u32 {
        let neights: [i32; 2] = [1, self.width as i32];
        let mut count = 0;

        for curr in 0..(self.width as i32 * (self.height as i32)) {
            let curr_dir = match self.pipes[curr as usize].kind {
                0 => 0b00010001u8,
                1 => 0b00110011,
                2 => 0b01010101,
                3 => 0b11101110,
                4 => 0b11111111,
                _ => 0,
            }
            .rotate_left(self.pipes[curr as usize].dir as u32);

            for i in 0..2 {
                let next = (curr + neights[i]) as usize;
                if (curr % self.width as i32 - next as i32 % self.width as i32).abs() <= 1
                    && next < self.regions.len()
                {
                    let next_dir = match self.pipes[next as usize].kind {
                        0 => 0b00010001u8,
                        1 => 0b00110011,
                        2 => 0b01010101,
                        3 => 0b11101110,
                        4 => 0b11111111,
                        _ => 0,
                    }
                    .rotate_left(self.pipes[next as usize].dir as u32 + 2);

                    // If curr and next have a paried connection, count it
                    if (curr_dir & (1 << i) != 0) && (next_dir & (1 << i) != 0) {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn add_life(&mut self, value: i32) {
        self.life_dirty = true;
        self.life = (self.life as i32 + value).max(0) as u32;
        if self.life == 0 && self.win_anim == 0.0 {
            self.trigger_lose();
        }
    }

    fn trigger_lose(&mut self) {
        self.lose_anim = 1.0;
        self.win_sprite.set_uv_rect(atlas::YOU_LOSE);
        self.win_sprite
            .set_size(atlas::YOU_LOSE[2] / atlas::YOU_LOSE[3], 1.0);
        self.win_sprite.set_color([255, 0, 0, 255]);
        self.win_sprite.set_angle(0.0);
        self.slow_down_effect.store(true, Ordering::Relaxed);
    }

    fn trigger_win(&mut self) {
        self.win_anim = 1.0;
        self.win_sprite.set_uv_rect(atlas::YOU_WIN);
        self.win_sprite
            .set_size(atlas::YOU_WIN[2] / atlas::YOU_WIN[3], 1.0);
        self.win_sprite.set_color([255, 255, 255, 255]);
        if self.sound_effects {
            crate::audio_engine()
                .new_sound(WavDecoder::new(Cursor::new(sounds::WHOOSH)).unwrap())
                .unwrap()
                .play();
        }
    }

    /// Receive the in world space coordinate of the mouse position.
    /// 'pressed' is 0 if none, 1 if is left button, 2 if is rigth button
    pub fn mouse_input(&mut self, x: f32, y: f32, pressed: u8) {
        if self.win_anim > 0.0 {
            self.highlight_sprite.pos[0] = -100.0;
            return;
        }
        if self.lose_anim > 0.0 {
            self.again_button.mouse_input(x, y);

            self.highlight_sprite.pos[0] = -100.0;
            if pressed == 1 && self.again_button.is_over {
                self.reset();
            }
            return;
        }

        let x = ((x + self.width as f32 / self.height as f32) / 2.0 * self.height as f32).floor()
            as i32;
        let y = ((y + 1.0) / 2.0 * self.height as f32).floor() as i32;
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            if pressed == 0 {
                self.highlight_sprite.pos = [
                    2.0 * (x as f32 + 0.5) / self.width as f32 - 1.0,
                    2.0 * (y as f32 + 0.5) / self.height as f32 - 1.0,
                ];
            } else {
                let i = y * self.width as i32 + x;
                self.click_count += 1;
                self.pipes[i as usize].click(pressed != 1, self.sound_effects);
                self.update_regions(i);
                let new_max = self.count_connections();
                if new_max > self.level_score {
                    self.score += new_max - self.level_score;
                    self.level_score = new_max;
                    self.score_dirty = true;
                }
                if self.check_is_done() {
                    self.trigger_win();
                }
                self.add_life(-1);
            }
        } else {
            self.highlight_sprite.pos[0] = -100.0;
        }
    }

    pub fn animate(&mut self, dt: f32) {
        if self.life_dirty {
            let w = self.life_text.get_width().max(self.score_text.get_width()) + 0.03;
            self.life_number = utils::number_to_sprites(
                self.life,
                self.life_text.pos[0] - self.life_text.get_width() / 2.0 + w,
                self.life_text.pos[1],
                self.life_text.get_height(),
                [255, 0, 0, 255],
                false,
                self.texture,
            );
        }

        if self.score_dirty {
            let w = self.life_text.get_width().max(self.score_text.get_width()) + 0.03;
            self.score_number = utils::number_to_sprites(
                self.score,
                self.score_text.pos[0] - self.score_text.get_width() / 2.0 + w,
                self.score_text.pos[1],
                self.score_text.get_height(),
                [255, 0, 0, 255],
                false,
                self.texture,
            );
        }

        for pipe in self.pipes.iter_mut() {
            pipe.animate(dt);
        }
        if self.win_anim == 0.0 && self.lose_anim == 0.0 {
            self.life_time -= dt;
            if self.life_time < 0.0 {
                self.add_life(-1);
                self.life_time += 0.5;
            }
        } else if self.win_anim > 0.0 {
            self.win_anim = (self.win_anim - dt * 0.5).max(0.0);

            let x = ((self.win_anim - 0.5) * PI).tan() * 0.5;
            let angle = lerp(x, 0.0, PI / 4.0);

            self.win_sprite.set_angle(angle);
            self.win_sprite.set_position(x, 0.0);

            if self.win_anim == 0.0 {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    use std::io::Write;
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .append(true)
                        .create(true)
                        .open("log.txt")
                        .unwrap();
                    writeln!(
                        file,
                        "{},{},{},{},{}",
                        self.width,
                        self.height,
                        self.expect_min_click_count,
                        self.click_count,
                        self.level_start.elapsed().as_secs_f32()
                    )
                    .unwrap();
                }
                self.new_level(self.width + 1, self.height + 1);
            }
        } else if self.lose_anim > 0.0 {
            self.again_button.update(dt);
            self.lose_anim = (self.lose_anim - dt * 0.5).max(f32::MIN_POSITIVE);

            let y = -0.2 * (self.lose_anim * self.lose_anim) / (self.lose_anim - 1.0);
            self.win_sprite.set_position(0.0, -y - 0.4);

            if self.lose_anim < 0.5 {
                let t = self.lose_anim * 2.0;
                let y = -0.4 * (t * t * 4.0) / (t - 1.0);
                self.score_text.set_position(0.0, y + 0.5);
                self.again_button.sprite.set_position(0.0, y * 3.0 + 0.7);
            } else {
                let t = self.lose_anim - 1.0;
                let d = t * t / (t + 0.5);
                if self.score_text.get_x() > 1.0 {
                    self.score_text
                        .set_position(1.13 + self.life_text.get_width() / 2.0 + d, -0.5);
                } else {
                    self.score_text.set_position(0.0, -1.3 - d);
                }
            }
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        if width > height {
            self.life_text
                .set_position(1.13 + self.life_text.get_width() / 2.0, -0.3);
            self.score_text
                .set_position(1.13 + self.score_text.get_width() / 2.0, -0.0);
        } else {
            self.life_text.set_position(-0.9, -1.3);
            self.score_text.set_position(0.0, -1.3);
        }
    }

    pub fn get_sprites(&self) -> Vec<SpriteInstance> {
        let mut sprites = Vec::with_capacity(self.pipes.len() + self.life_number.len() + 3);
        sprites.push(self.highlight_sprite.clone());
        for pipe in self.pipes.iter() {
            sprites.push(pipe.sprite.clone());
        }
        sprites.push(self.life_text.clone());
        sprites.extend(self.life_number.iter().cloned());
        sprites.push(self.score_text.clone());
        sprites.extend(self.score_number.iter().cloned());
        if self.win_anim > 0.0 {
            sprites.push(self.win_sprite.clone());
        }
        if self.lose_anim > 0.0 {
            sprites.push(self.win_sprite.clone());
            if self.lose_anim < 0.5 {
                sprites.push(self.again_button.sprite.clone());
            }
        }
        sprites
    }
}

struct Button {
    sprite: SpriteInstance,
    height: f32,
    bounds: [f32; 4],
    anim: f32,
    is_over: bool,
}
impl Button {
    fn new(sprite: SpriteInstance, bounds: [f32; 4]) -> Self {
        Self {
            height: sprite.get_height(),
            sprite,
            bounds,
            anim: 0.0,
            is_over: false,
        }
    }

    fn mouse_input(&mut self, x: f32, y: f32) {
        let left = self.sprite.get_x() + self.bounds[0];
        let rigth = self.sprite.get_x() + self.bounds[1];
        let top = self.sprite.get_y() + self.bounds[2];
        let bottom = self.sprite.get_y() + self.bounds[3];

        self.is_over = x > left && x < rigth && y > top && y < bottom;
    }

    fn update(&mut self, dt: f32) {
        let s = self.anim * 0.1 + 1.0;
        self.sprite.set_heigh_prop(self.height * s);
        if self.is_over {
            self.anim = (self.anim + dt * 6.0).min(1.0);
        } else {
            self.anim = (self.anim - dt * 6.0).max(0.0);
        }
    }
}

pub struct Game<R: Rng, S: SpriteRender> {
    camera: Camera,
    render: S,
    background_painel: SpriteInstance,
    start_button: Button,
    close_button: Button,
    music_button: Button,
    audio_button: Button,
    back_button: Button,
    board: GameBoard<R>,
    in_menu: bool,
}
impl<R: Rng, S: SpriteRender> Game<R, S> {
    pub fn new(
        rng: R,
        music: Sound,
        slow_down_effect: Arc<AtomicBool>,
        camera: Camera,
        mut render: S,
    ) -> Self {
        let texture = {
            let image =
                image::load_from_memory(include_bytes!(concat!(env!("OUT_DIR"), "/atlas.png")))
                    .unwrap()
                    .to_rgba8();
            Texture::new(image.width(), image.height())
                .data(image.into_raw().as_slice())
                .create(&mut render)
                .unwrap()
        };
        Self {
            camera,
            render,
            background_painel: SpriteInstance::new(0.0, 0.0, 2.2, 2.2, texture, atlas::PAINEL),
            start_button: Button::new(
                SpriteInstance::new_height_prop(0.0, 0.0, 0.5, texture, atlas::START_BUTTON)
                    .with_color([0, 230, 0, 255]),
                [-0.62, 0.62, -0.20, 0.20],
            ),
            close_button: Button::new(
                SpriteInstance::new_height_prop(0.0, 0.5, 0.25, texture, atlas::CLOSE_BUTTON)
                    .with_color([0, 150, 0, 255]),
                [-0.65, 0.65, -0.20, 0.20],
            ),
            back_button: Button::new(
                SpriteInstance::new_height_prop(0.0, 0.5, 0.25, texture, atlas::ARROW)
                    .with_color([0, 240, 0, 255]),
                [-0.12, 0.12, -0.12, 0.12],
            ),
            music_button: Button::new(
                SpriteInstance::new_height_prop(0.0, 0.5, 0.15, texture, atlas::MUSIC)
                    .with_color([0, 240, 0, 255]),
                [-0.07, 0.07, -0.07, 0.07],
            ),
            audio_button: Button::new(
                SpriteInstance::new_height_prop(0.0, 0.5, 0.15, texture, atlas::SOUND)
                    .with_color([0, 240, 0, 255]),
                [-0.07, 0.07, -0.07, 0.07],
            ),
            board: GameBoard::new(texture, rng, music, slow_down_effect),
            in_menu: true,
        }
    }

    pub fn update(&mut self, dt: f32, input: &Input) {
        let (mouse_x, mouse_y) = self
            .camera
            .position_to_word_space(input.mouse_x, input.mouse_y);

        self.music_button.mouse_input(mouse_x, mouse_y);
        self.audio_button.mouse_input(mouse_x, mouse_y);
        if input.mouse_left_state == 3 {
            if self.music_button.is_over {
                if self.music_button.anim == 0.0 {
                    self.music_button.sprite.set_uv_rect(atlas::MUSIC_OFF);
                    self.music_button.anim = 1.0;
                    self.board.music.set_volume(0.0);
                } else {
                    self.music_button.sprite.set_uv_rect(atlas::MUSIC);
                    self.music_button.anim = 0.0;
                    self.board.music.set_volume(1.0);
                }
            }
            if self.audio_button.is_over {
                if self.audio_button.anim == 0.0 {
                    self.audio_button.sprite.set_uv_rect(atlas::SOUND_OFF);
                    self.audio_button.anim = 1.0;
                    self.board.sound_effects = false;
                } else {
                    self.audio_button.sprite.set_uv_rect(atlas::SOUND);
                    self.audio_button.anim = 0.0;
                    self.board.sound_effects = true;
                }
            }
        }

        if self.in_menu {
            self.start_button.mouse_input(mouse_x, mouse_y);
            self.start_button.update(dt);
            if input.mouse_left_state == 3 && self.start_button.is_over {
                self.in_menu = false;
                self.update_layout();
                self.board.reset();
                if self.board.sound_effects {
                    crate::audio_engine()
                        .new_sound(WavDecoder::new(Cursor::new(sounds::CONFIRM)).unwrap())
                        .unwrap()
                        .play();
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.close_button.mouse_input(mouse_x, mouse_y);
                self.close_button.update(dt);
                if input.mouse_left_state == 3 && self.close_button.is_over {
                    std::process::exit(0);
                }
            }
        } else {
            self.back_button.mouse_input(mouse_x, mouse_y);
            self.back_button.update(dt);

            if input.mouse_left_state == 3 && self.back_button.is_over {
                self.in_menu = true;
                self.update_layout();
                if self.board.sound_effects {
                    crate::audio_engine()
                        .new_sound(WavDecoder::new(Cursor::new(sounds::NEGATE)).unwrap())
                        .unwrap()
                        .play();
                }
            }

            self.board.mouse_input(
                mouse_x,
                mouse_y,
                if input.mouse_left_state == 3 {
                    1
                } else if input.mouse_rigth_state == 3 {
                    2
                } else {
                    0
                },
            );
            self.board.animate(dt);
        }
    }

    pub fn render(&mut self, window_id: WindowId) {
        let sprites = self.get_sprites();
        self.render
            .render(window_id)
            .clear_screen(&[0.0f32, 0.25, 0.0, 1.0])
            .draw_sprites(&mut self.camera, &sprites)
            .finish();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>, window_id: WindowId) {
        self.camera.resize(size.width, size.height);
        self.render.resize(window_id, size.width, size.height);
        self.update_layout();
    }

    pub fn update_layout(&mut self) {
        let prop = self.camera.width() as f32 / self.camera.height() as f32;
        if !self.in_menu {
            if prop > 1.0 {
                // landscape
                if prop > 1280.0 / 720.0 {
                    self.camera.set_position(0.0, 0.0);
                } else if prop > 1280.0 / 720.0 / 2.0 + 0.5 {
                    self.camera.set_position(
                        -self.camera.width() as f32 / 2.0 + 1.1 * 1280.0 / 720.0,
                        0.0,
                    );
                } else {
                    self.camera
                        .set_position(self.camera.width() as f32 / 2.0 - 1.1, 0.0);
                }
            } else {
                // portrait
                if prop < 720.0 / 1280.0 {
                    self.camera.set_position(0.0, 0.0);
                } else if prop < 1.0 / (1280.0 / 720.0 / 2.0 + 0.5) {
                    self.camera.set_position(
                        0.0,
                        self.camera.height() as f32 / 2.0 - 1.1 * 1280.0 / 720.0,
                    );
                } else {
                    self.camera
                        .set_position(0.0, -self.camera.height() as f32 / 2.0 + 1.1);
                }
            }
        } else {
            self.camera.set_position(0.0, 0.0);
        }

        let width = self.camera.width();
        let height = self.camera.height();
        self.board.resize(width, height);

        if self.in_menu {
            self.music_button.sprite.set_position(0.95, 0.95);
            self.audio_button.sprite.set_position(0.75, 0.95);
        } else {
            let right_side = width / 2.0 + self.camera.get_position().0;
            if prop > 1.0 {
                self.music_button
                    .sprite
                    .set_position(right_side - 0.15, 0.95);
                self.audio_button
                    .sprite
                    .set_position(right_side - 0.35, 0.95);
            } else {
                let top_side = -height / 2.0 + self.camera.get_position().1;
                self.music_button
                    .sprite
                    .set_position(right_side - 0.15, top_side + 0.15);
                self.audio_button
                    .sprite
                    .set_position(right_side - 0.35, top_side + 0.15);
            }
        }

        if prop > 1.0 {
            let x;
            if prop > 1280.0 / 720.0 {
                x = -1.1 - (1280.0 / 720.0 - 1.0) * 1.1 / 2.0;
            } else if prop > 1280.0 / 720.0 / 2.0 + 0.65 {
                let l = -width + 1.1 * 1280.0 / 720.0;
                x = (l - 1.1) / 2.0;
            } else if prop > 1280.0 / 720.0 / 2.0 + 0.5 {
                x = 1.1 * 1280.0 / 720.0 - 0.2;
            } else {
                x = width - 1.1 - 0.2;
            }
            self.back_button.sprite.set_position(x, -0.9);
        } else {
            self.back_button.sprite.set_position(-0.9, -1.6);
        }
    }

    pub fn get_sprites(&mut self) -> Vec<SpriteInstance> {
        if self.in_menu {
            vec![
                self.background_painel.clone(),
                self.music_button.sprite.clone(),
                self.audio_button.sprite.clone(),
                self.start_button.sprite.clone(),
                #[cfg(not(target_arch = "wasm32"))]
                self.close_button.sprite.clone(),
            ]
        } else {
            let mut vec = vec![
                self.back_button.sprite.clone(),
                self.music_button.sprite.clone(),
                self.audio_button.sprite.clone(),
                self.background_painel.clone(),
            ];
            vec.append(&mut self.board.get_sprites());
            vec
        }
    }
}
