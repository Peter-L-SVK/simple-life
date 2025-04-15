use rand::Rng;
use piston_window::*;
use crate::genetics::Genetics;
use crate::food::Food;
use crate::{BASE_BEING_SIZE, ENERGY_DECAY, WINDOW_SIZE};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeingType {
    Herbivore,
    Carnivore,
    Omnivore,
}

#[derive(Clone, PartialEq)]
pub struct Being {
    pub x: f64,
    pub y: f64,
    pub color: [f32; 4],
    pub energy: f32,
    pub being_type: BeingType,
    pub genetics: Genetics,
    pub age: u32,
    pub max_age: u32,
}

impl Being {
   pub fn new(x: f64, y: f64, being_type: BeingType) -> Self {
        let genetics = Genetics::new_random(being_type);
        
        let (color, max_age) = match being_type {
            BeingType::Herbivore => ([0.0, 0.0, 1.0, 1.0], 3000),
            BeingType::Carnivore => ([1.0, 0.0, 0.0, 1.0], 2000),
            BeingType::Omnivore => ([1.0, 0.5, 0.0, 1.0], 2500),
        };

        Being {
            x,
            y,
            color,
            energy: 1.0,
            being_type,
            genetics,
            age: 0,
            max_age,
        }
    }

    pub fn size(&self) -> f64 {
        BASE_BEING_SIZE * self.genetics.size as f64
    }

    pub fn update(&mut self, beings: &[Being], foods: &[Food]) -> (Vec<usize>, Option<Being>) {
	// Filter beings based on type before processing
	let filtered_beings: Vec<&Being> = match self.being_type {
            BeingType::Herbivore => Vec::new(),  // Herbivores don't need other beings
            BeingType::Carnivore => beings.iter()
		.filter(|b| matches!(b.being_type, BeingType::Herbivore | BeingType::Omnivore))
		.collect(),
            BeingType::Omnivore => beings.iter()
		.filter(|b| b.being_type != self.being_type)
		.collect(),
	};
        
        // Original update logic using filtered_beings instead of beings
        let mut rng = rand::rng();
        self.age += 1;
        self.energy -= ENERGY_DECAY * (self.genetics.size + self.genetics.speed);  // Lose energy based on size and speed
        
        let perception_range = self.genetics.perception as f64;  // Movement based on perception
        let mut eaten_food_indices = Vec::new();
        let mut new_being = None;
        
        match self.being_type {
            BeingType::Herbivore => {
                self.update_herbivore(foods, perception_range, &mut rng, &mut eaten_food_indices)
            },
            BeingType::Carnivore => {
                if let Some(prey) = self.update_carnivore(&filtered_beings, perception_range, &mut rng) {
                    return (vec![], Some(prey));
                }
            },
            BeingType::Omnivore => {
                if let Some((prey, food_indices)) = self.update_omnivore(&filtered_beings, foods, perception_range, &mut rng) {
                    if let Some(p) = prey {
                        return (food_indices, Some(p));
                    }
                    eaten_food_indices = food_indices;
                }
            },
        }
        
        self.x = self.x.max(0.0).min(WINDOW_SIZE - self.size());
        self.y = self.y.max(0.0).min(WINDOW_SIZE - self.size());
        
        if self.can_replicate() {
            new_being = Some(self.replicate());
        }
        
        (eaten_food_indices, new_being)
    }

    pub fn update_herbivore(
	&mut self,
	foods: &[Food],
	perception_range: f64,
	rng: &mut impl Rng,
	eaten_food_indices: &mut Vec<usize>,
    ) {
	// Only look at food, ignore other beings completely
	if let Some((idx, nearest_food)) = foods.iter().enumerate().min_by_key(|(_, f)| {
            let dx = f.x - self.x;
            let dy = f.y - self.y;
            ((dx * dx + dy * dy) * 1000.0) as i32
	}) {
            let dx = nearest_food.x - self.x;
            let dy = nearest_food.y - self.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance < perception_range {
		self.x += dx / distance * self.genetics.speed as f64 * 1.5;
		self.y += dy / distance * self.genetics.speed as f64 * 1.5;
		
		if distance < self.size() / 2.0 + 2.5 {
                    eaten_food_indices.push(idx);
                    self.energy += nearest_food.energy;
		}
            } else {
		self.random_movement(rng);
            }
	} else {
            self.random_movement(rng);
	}
    }
    
    pub fn update_carnivore(&mut self, beings: &[&Being], perception_range: f64, rng: &mut impl Rng) -> Option<Being> {
	// Find ALL potential prey in perception range (not just nearest)
	let mut potential_prey: Vec<_> = beings.iter()
            .filter(|&&b| b.size() < self.size() * 1.1)  // Slightly larger threshold
            .filter(|&&b| {
		let dx = b.x - self.x;
		let dy = b.y - self.y;
		(dx * dx + dy * dy).sqrt() < perception_range * 1.5  // Larger detection range
            })
            .collect();
	
	// If we found prey
	if !potential_prey.is_empty() {
            // Sort by distance AND energy (prioritize closer, higher energy prey)
            potential_prey.sort_by(|&&a, &&b| {
		let dist_a = (a.x - self.x).powi(2) + (a.y - self.y).powi(2);
		let dist_b = (b.x - self.x).powi(2) + (b.y - self.y).powi(2);
		let weight_a = dist_a * (1.1 - a.energy as f64);
		let weight_b = dist_b * (1.1 - b.energy as f64);
		weight_a.partial_cmp(&weight_b).unwrap()
            });
	    
            let target = potential_prey[0];
            let dx = target.x - self.x;
            let dy = target.y - self.y;
            let distance = (dx * dx + dy * dy).sqrt();
	    
            // More aggressive chasing
            let speed_multiplier = if distance < perception_range { 3.5 } else { 2.5 };
            self.x += dx / distance * self.genetics.speed as f64 * speed_multiplier;
            self.y += dy / distance * self.genetics.speed as f64 * speed_multiplier;
	    
            if distance < self.size() / 2.0 + target.size() / 2.0 {
		self.energy += target.energy * 0.95;
		return Some((*target).clone());
            }
	} else {
            // More purposeful wandering when no prey is visible
            self.x += rng.random_range(-1.0..1.0) * self.genetics.speed as f64 * 1.5;
            self.y += rng.random_range(-1.0..1.0) * self.genetics.speed as f64 * 1.5;
	}
	
	None
    }
    
    pub fn update_omnivore(
	&mut self,
	beings: &[&Being],
	foods: &[Food],
	perception_range: f64,
	rng: &mut impl Rng,
    ) -> Option<(Option<Being>, Vec<usize>)> {
	let mut eaten_food_indices = Vec::new();

	// Alternate between food and smaller beings
	if rng.random_bool(0.7) {
            if let Some(target) = beings.iter()
		.filter(|&&b| b.size() < self.size() * 0.9)
		.min_by_key(|&&b| {
                    let dx = b.x - self.x;
                    let dy = b.y - self.y;
                    ((dx * dx + dy * dy) * (1.0 + b.energy as f64)) as i32
		})
            {
		let dx = target.x - self.x;
		let dy = target.y - self.y;
		let distance = (dx * dx + dy * dy).sqrt();
		
		if distance < perception_range {
                    self.x += dx / distance * self.genetics.speed as f64 * 2.2;
                    self.y += dy / distance * self.genetics.speed as f64 * 2.2;
                    
                    if distance < self.size() / 2.0 + target.size() / 2.0 {
			self.energy += target.energy * 0.85;
			return Some((Some((*target).clone()), vec![]));  
                    }
		}
            }
	} else {
            if let Some((idx, nearest_food)) = foods.iter().enumerate()
		.min_by_key(|(_, f)| {
                    let dx = f.x - self.x;
                    let dy = f.y - self.y;
                    ((dx * dx + dy * dy) * 1000.0) as i32
		}) 
            {
		let dx = nearest_food.x - self.x;
		let dy = nearest_food.y - self.y;
		let distance = (dx * dx + dy * dy).sqrt();
		
		if distance < perception_range * 1.2 {
                    self.x += dx / distance * self.genetics.speed as f64 * 1.8;
                    self.y += dy / distance * self.genetics.speed as f64 * 1.8;
                    
                    if distance < self.size() / 2.0 + 2.5 {
			eaten_food_indices.push(idx);
			self.energy += nearest_food.energy * 1.2;
                    }
		}
            }
	}
	
	self.random_movement(rng);
	Some((None, eaten_food_indices))
    }
    
    pub fn random_movement(&mut self, rng: &mut impl Rng) {
        self.x += rng.random_range(-1.0..1.0) * self.genetics.speed as f64;
        self.y += rng.random_range(-1.0..1.0) * self.genetics.speed as f64;
    }

    pub fn can_replicate(&self) -> bool {
        let mut rng = rand::rng();
        let base_chance = match self.being_type {
            BeingType::Carnivore => 0.0016,
            BeingType::Omnivore => 0.0013,
            BeingType::Herbivore => 0.0011,
        };
        
        self.energy > 0.8 &&
            rng.random_range(0.0..1.0) < (base_chance * self.genetics.reproduction_rate) &&
            self.age > 80 &&
            self.age < self.max_age
    }

    pub  fn replicate(&mut self) -> Being {
        let mut child = self.clone(); // Ensure this copies all fields properly
        let mut rng = rand::rng();
        
        child.x += rng.random_range(-20.0..20.0);
        child.y += rng.random_range(-20.0..20.0);
        child.energy = self.energy * 0.5;
        child.genetics = self.genetics.mutate();
        child.age = 0;
        self.energy *= 0.5;
        
        child
    }

    pub fn draw(&self, transform: math::Matrix2d, g: &mut G2d) {
        let size = self.size();
        rectangle(
            self.color,
            [self.x, self.y, size, size],
            transform,
            g,
        );
    }
}
