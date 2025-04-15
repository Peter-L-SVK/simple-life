use piston_window::*;
use rand::Rng;

use crate::WINDOW_SIZE;

#[derive(Clone, PartialEq)]
pub struct Food {
    pub x: f64,
    pub y: f64,
    pub energy: f32,
}

impl Food {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        Food {
            x: rng.random_range(0.0..WINDOW_SIZE),
            y: rng.random_range(0.0..WINDOW_SIZE),
            energy: rng.random_range(0.3..0.7),
        }
    }
    
    pub fn draw(&self, transform: math::Matrix2d, g: &mut G2d) {
	let size = 5.0;
	rectangle(
            [0.0, 1.0, 0.0, 1.0],  // Pure green
            [self.x, self.y, size, size],
            transform,
            g,
	);
    }
}
