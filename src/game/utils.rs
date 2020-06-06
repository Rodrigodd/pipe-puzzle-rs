use sprite_render::SpriteInstance;
use super::atlas::NUMBERS;

pub fn number_to_sprites(number: u32, x: f32, y: f32, scale: f32, color: [u8; 4], center: bool, texture: u32) -> Vec<SpriteInstance> {
    let string = number.to_string();
    let mut sprites = Vec::with_capacity(string.len());
    let mut pos = scale/2.0;
    for c in string.chars() {
        let mut sprite = SpriteInstance::new(x+pos, y, scale, scale, texture, NUMBERS[c as usize - '0' as usize]);
        sprite.set_color(color);
        sprites.push(sprite);
        pos += scale*0.8;
    }
    if center {
        for sprite in sprites.iter_mut() {
            sprite.pos[0] -= pos/2.0;
        }
    }
    sprites
}