use sprite_render::SpriteInstance;

use rand::Rng;
use rand::seq::index::sample;

use std::f32::consts::PI;
use std::collections::HashMap;

use ezing::*;
fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + (b-a)*t
}

const COLORS: &[[f32;4]] = &[
    [1.0, 0.0, 0.0, 1.0,], [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0], // red, green, ~blue
    [1.0, 1.0, 0.0, 1.0,], [1.0, 0.0, 1.0, 1.0], [0.0, 1.0, 1.0, 1.0], // yellow, purple, cyan
    [1.0, 0.5, 0.0, 1.0,], [1.0, 1.0, 1.0, 1.0], [0.7, 0.7, 0.7, 1.0], // orange, white, gray
    [0.6, 0.1, 0.0, 1.0,], [0.7, 0.7, 0.0, 1.0], [0.3, 0.3, 0.3, 1.0], // brown, dark_yellow, black
];
const UV_RECTS: &[[f32;4]] = &[
    [0.0, 0.0, 0.5, 1.0/3.0],
    [0.5, 0.0, 0.5, 1.0/3.0],
    [0.0, 1.0/3.0, 0.5, 1.0/3.0],
    [0.5, 1.0/3.0, 0.5, 1.0/3.0],
    [0.0, 2.0/3.0, 0.5, 1.0/3.0],
    [0.5, 2.0/3.0, 0.5, 1.0/3.0]
];

struct Pipe {
    dir: u8,
    kind: u8,
    sprite: SpriteInstance,
    angle: f32,
    size: f32,
    previous_angle: f32,
    anim_time: f32,
    color_time: f32,
    previous_color: [f32; 4],
    target_color: u16,
}
impl Pipe {
    fn new(x: f32, y: f32, size: f32, texture: u32, kind: u8, dir: u8) -> Self {
        Self {
            dir,
            kind,
            sprite: SpriteInstance::new(x, y, size, size, texture, UV_RECTS[kind as usize]),
            angle: 0.0,
            size,
            previous_angle: 0.0,
            anim_time: 0.000001,
            color_time: 0.000001,
            previous_color: [0.0; 4],
            target_color: 0,
        }
    }

    fn animate(&mut self, dt: f32) {
        if self.anim_time != 0.0 {
            self.anim_time = (self.anim_time - dt*0.7).max(0.0);
            let d = ((self.dir as f32 * PI/2.0) - self.previous_angle + PI).rem_euclid(2.0*PI) - PI;
            let t = elastic_out(1.0 - self.anim_time);
            self.angle = self.previous_angle + lerp(t, 0.0, d);
            self.sprite.set_transform(self.size, self.size, self.angle);
        }
        if self.color_time != 0.0 {
            self.color_time = (self.color_time - dt*3.0).max(0.0);
            let t = 1.0 - self.color_time;
            let color = [
                lerp(t, self.previous_color[0], COLORS[self.target_color as usize % COLORS.len()][0]),
                lerp(t, self.previous_color[1], COLORS[self.target_color as usize % COLORS.len()][1]),
                lerp(t, self.previous_color[2], COLORS[self.target_color as usize % COLORS.len()][2]),
                1.0,
            ];
            self.sprite.set_color(color);
        }
    }

    /// Is called when there is a mouse click over self.
    /// When right is true it is a right mouse button click, otherwise is left.
    fn click(&mut self, right: bool) {
        if right {
            self.dir = (self.dir + 1)%4;
        } else {
            self.dir = (self.dir + 4 - 1)%4;
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

pub struct Game<R: Rng> {
    start_button: SpriteInstance,
    close_button: SpriteInstance,
    board: GameBoard<R>,
    in_menu: bool,
}
impl<R: Rng> Game<R> {

    pub fn new(texture: u32, rng: R) -> Self {
        Self {
            start_button: SpriteInstance::new(0.0, -0.5, 1.4, 0.5, texture, UV_RECTS[4]),
            close_button: SpriteInstance::new(0.0, 0.5, 1.0, 0.5, texture, UV_RECTS[4]),
            board: GameBoard::new(3, 3, texture, rng),
            in_menu: true,
        }
    }

    pub fn handle_click(&mut self, x: f32, y: f32, right: bool) {
        if self.in_menu {
            if y < 0.0 {
                self.in_menu = false;
            }
        } else {
            self.board.handle_click(x, y, right);
        }
    
    }

    pub fn update(&mut self, dt: f32) {
        if self.in_menu {
            // if y < 0.0 {
            //     self.start_button.set_color([1.0, 0.4, 0.4, 1.0]);
            //     self.close_button.set_color([1.0, 1.0, 1.0, 1.0]);
            // } else {
            //     self.close_button.set_color([1.0, 0.4, 0.4, 1.0]);
            //     self.start_button.set_color([1.0, 1.0, 1.0, 1.0]);
            // }
        } else {
            self.board.animate(dt);
        }
    }

    pub fn get_sprites(&mut self) -> Vec<SpriteInstance> {
        if self.in_menu {
            vec![self.start_button.clone(), self.close_button.clone()]
        } else {
            self.board.get_sprites()
        }
    }
}

struct GameBoard<R: Rng> {
    width: u8,
    height: u8,
    pipes: Vec<Pipe>,
    regions: Vec<u16>,
    region_id_pool: Vec<u16>,
    number_regions: u16,
    region_size: HashMap<u16, u16>,
    win_anim: f32,
    win_sprite: SpriteInstance,
    texture: u32,
    rng: R,
}
impl<R: Rng> GameBoard<R> {

    pub fn new(width: u8, height: u8, texture: u32, rng: R) -> Self {
        
        let mut this = Self {
            width,
            height,
            pipes: Vec::new(),
            regions: Vec::new(),
            region_id_pool: Vec::new(),
            number_regions: 0,
            region_size: HashMap::new(),
            win_anim: 0.0,
            win_sprite: SpriteInstance::new(0.0, 0.0, 2.0, 2.0, texture, UV_RECTS[5]),
            texture,
            rng,
        };
        this.new_level(width, height);
        this
    }

    fn new_level(&mut self, width: u8, height: u8) {
        self.width = width;
        self.height = height;
        self.region_id_pool.clear();
        self.region_size.clear();
        self.number_regions = 0;
        self.pipes = Vec::with_capacity(width as usize * height as usize);
        let maze = Self::gen_maze(width, height, &mut self.rng);
        let mut i = 0usize;
        let size = 2.0 / self.height as f32;
        
        for y in 0..height {
            for x in 0..width {
                let (kind, _dir) = match maze[i] {
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
                // answer.push(dir);
                self.pipes.push(Pipe::new(
                    (x as f32 - width as f32 / 2.0) * size + size/2.0,
                    (y as f32 - height as f32 / 2.0) * size + size/2.0,
                    size,
                    self.texture, 
                    kind, self.rng.gen_range(0, 1))
                );
                i+=1;
            }
        }
        self.trace_regions();
    }

    // preference == 0 mean no preference
    fn next_region_id(&mut self, preference: u16) -> u16 {

        let ret = if self.region_id_pool.is_empty() {
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
                i+=1;
            }
        };
        ret
    }

    // If a region is not more valid, it is add back to the pool,
    // and its region size is removed
    fn add_region_to_pool(&mut self, region: u16) {
        if region != 0 && !self.region_id_pool.iter().any(|&x| x == region) {
            self.region_id_pool.push(region);
            self.region_size.remove(&region);
        }
    }

    fn trace_region(&mut self, start: i32, region: u16) {
        let neights: [i32; 4] = [1, self.width as i32, -1, -(self.width as i32)];
        let mut explore: Vec<i32> = vec![start];
        self.pipes[start as usize].change_color(region);
        self.regions[start as usize] = region;
        let mut visited = vec![false; self.width as usize * self.height as usize];
        visited[start as usize] = true;
        let mut count = 1u16;
        while let Some(curr) = explore.pop() {
            let curr_dir = match self.pipes[curr as usize].kind {
                0 => 0b00010001u8,
                1 => 0b00110011,
                2 => 0b01010101,
                3 => 0b11101110,
                4 => 0b11111111,
                _ => 0,
            }.rotate_left(self.pipes[curr as usize].dir as u32);

            for i in 0..4 {
                let next = (curr + neights[i]) as usize;
                if (curr % self.width as i32 - next as i32 % self.width as i32).abs() <= 1
                && next < self.regions.len()
                && !visited[next] {      // if it is inexpored
                    let next_dir = match self.pipes[next as usize].kind {
                        0 => 0b00010001u8,
                        1 => 0b00110011,
                        2 => 0b01010101,
                        3 => 0b11101110,
                        4 => 0b11111111,
                        _ => 0,
                    }.rotate_left(self.pipes[next as usize].dir as u32 + 2);
                    if curr_dir & (1<<i) & next_dir != 0 {
                        explore.push(next as i32);
                        self.regions[next] = region;
                        visited[next] = true;
                        self.pipes[next].change_color(region);
                        count += 1;
                    }
                }
            }
        }

        self.region_size.insert(region, count);

    }

    fn trace_regions(&mut self) {
        self.regions = vec![0u16; self.width as usize * self.height as usize];
        
        let mut region;
        for i in 0..(self.width as usize * self.height as usize) {
            if self.regions[i] == 0 {
                region = self.next_region_id(0);
                self.trace_region(i as i32, region);
            }
        }
    }

    fn update_regions(&mut self, i: i32) {
        let neights: [i32; 4] = [1, self.width as i32, -1, -(self.width as i32)];

        let mut to_check: Vec<(usize, u16)> = Vec::with_capacity(5);

        to_check.push((i as usize, self.regions[i as usize]));
        self.regions[i as usize] = 0;

        for n in 0..4 {
            let next = (i + neights[n]) as usize;
            if (i % self.width as i32 - next as i32 % self.width as i32).abs() <= 1
            && next < self.regions.len() {
                to_check.push((next, self.regions[next]));
                self.regions[next] = 0;
            }
        }
        
        to_check.sort_by_key(|(_, x)| {
            u16::max_value() - self.region_size.get(x).unwrap()
        });

        for &(_, region) in to_check.iter() {
            self.add_region_to_pool(region);
        }

        let mut region: u16;
        for (curr, preference) in to_check.into_iter() {
            if self.regions[curr] == 0 {
                region = self.next_region_id(preference);
                self.trace_region(curr as i32, region);
            }
        }
    }

    fn gen_maze(width: u8, height: u8, rng: &mut R) -> Box<[i8]> {
        let neights: [i32; 4] = [1, width as i32, -1, -(width as i32)];

        let mut grid = vec![0i8; width as usize * height as usize].into_boxed_slice();

        let start = rng.gen_range(0, grid.len());
        let mut path: Vec<i32> = vec![start as i32];
        grid[start] = 0;
        
        'path: while path.len() > 0 {
            let r = rng.gen_range(0, path.len());
            let curr = path[r];
            for i in sample(rng, 4, 4).iter() {
                let next = (curr + neights[i]) as usize;
                if (curr % width as i32 - next as i32 % width as i32).abs() <= 1
                && next < grid.len()
                && grid[next] == 0 {      // if it is inexpored
                    grid[curr as usize] |= 1 << i;  // set dir bitmask
                    grid[next] |= 1 << ((i+2)%4); // set dir bitmask
                    path.push(next as i32);         // add to the backtrack path
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
                    && grid[next] & (1 << ((i+2)%4)) == 0 {
                        grid[curr as usize] |= 1 << i;  // set dir bitmask
                        grid[next] |= 1 << ((i+2)%4); // set dir bitmask
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
                _ => print!(" ")

            };
            i+=1;
            if i%width as usize == 0 {
                println!();
            }
        }
        grid
    }

    fn check_is_done(&self) -> bool {
        let neights: [i32; 2] = [1, self.width as i32];

        // if there is more than one region, it is not done
        if self.region_size.len() > 1 {
            return false;
        }

        for curr in 0..self.width as i32 {
            let curr_dir = match self.pipes[curr as usize].kind {
                0 => 0b00010001u8,
                1 => 0b00110011,
                2 => 0b01010101,
                3 => 0b11101110,
                4 => 0b11111111,
                _ => 0,
            }.rotate_left(self.pipes[curr as usize].dir as u32);

            if curr_dir & (1<<3) != 0 { // if curr is connect to nowhere, it is not done
                return false;
            }
        }

        for curr in (0..(self.width as i32 * self.height as i32)).step_by(self.width as usize) {
            let curr_dir = match self.pipes[curr as usize].kind {
                0 => 0b00010001u8,
                1 => 0b00110011,
                2 => 0b01010101,
                3 => 0b11101110,
                4 => 0b11111111,
                _ => 0,
            }.rotate_left(self.pipes[curr as usize].dir as u32);

            if curr_dir & (1<<2) != 0 { // if curr is connect to nowhere, it is not done
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
            }.rotate_left(self.pipes[curr as usize].dir as u32);

            for i in 0..2 {
                let next = (curr + neights[i]) as usize;
                if (curr % self.width as i32 - next as i32 % self.width as i32).abs() <= 1
                && next < self.regions.len() {
                    let next_dir = match self.pipes[next as usize].kind {
                        0 => 0b00010001u8,
                        1 => 0b00110011,
                        2 => 0b01010101,
                        3 => 0b11101110,
                        4 => 0b11111111,
                        _ => 0,
                    }.rotate_left(self.pipes[next as usize].dir as u32 + 2);

                    // If curr and next have a unparied connection, it is not done
                    if (curr_dir & (1<<i) != 0) != (next_dir & (1<<i) != 0) {
                        return false;
                    }
                } else {
                    if curr_dir & (1<<i) != 0 { // if curr is connect to nowhere, it is not done
                        return false;
                    }
                }
            }
        }
        // if there is no pipe with unparied connection, it is done
        true
    }

    /// Receive the in world space coordinate of the mouse click.
    /// When right is true it is a right mouse button click, otherwise is left.
    pub fn handle_click(&mut self, x: f32, y: f32, right: bool) {
        if self.win_anim > 0.0 {
            return;
        }
        let x = ((x + self.width as f32 / self.height as f32)/2.0 * self.height as f32) as u16;
        let y = ((y + 1.0) / 2.0 * self.height as f32) as u16;
        if x < self.width as u16 && y < self.height as u16 {
            let i = y*self.width as u16 + x;
            self.pipes[i as usize].click(right);
            self.update_regions(i as i32);
            if self.check_is_done() {
                self.win_anim = 1.0;
            }
        }
    }

    pub fn animate(&mut self, dt: f32) {
        for pipe in self.pipes.iter_mut() {
            pipe.animate(dt);
        }
        if self.win_anim > 0.0 {
            self.win_anim = (self.win_anim - dt*0.5).max(0.0);

            // this is a function that:
            // f(1) = +infinite, 
            // f(0.5) = 0,
            // f(0) = -infinite,
            // f'(0.5) = 0.15 (derivative)
            // let x = (self.win_anim - 0.5) -
            //     ((1.0/self.win_anim)*(1.0/(self.win_anim-1.0)) + 4.0) 
            //     * if self.win_anim < 0.5 { -1.0 } else { 1.0 };
            let x = ((self.win_anim-0.5)*PI).tan()*0.5;
            let angle = lerp(x, 0.0, PI/4.0);

            self.win_sprite.set_transform(1.0, 1.0, angle);
            self.win_sprite.set_position(x, 0.0);

            if self.win_anim == 0.0 {
                self.new_level(self.width + 1, self.height  + 1);
            }
        }
    }

    pub fn get_sprites(&self) -> Vec<SpriteInstance> {
        let mut sprites= Vec::with_capacity(self.pipes.len());
        for pipe in self.pipes.iter() {
            sprites.push(pipe.sprite.clone());
        }
        if self.win_anim > 0.0 {
            sprites.push(self.win_sprite.clone());
        }

        sprites
    }
}