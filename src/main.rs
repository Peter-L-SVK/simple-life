use piston_window::*;
use rand::Rng;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

const WINDOW_SIZE: f64 = 800.0;
const BASE_BEING_SIZE: f64 = 10.0;
const MAX_BEINGS: usize = 220;
const MAX_FOOD: usize = 790;
const FOOD_SPAWN_RATE: f64 = 0.99;
const ENERGY_DECAY: f32 = 0.0000015;
const STATS_AREA_HEIGHT: f64 = 50.0; // New constant for stats area height
const TOTAL_WINDOW_HEIGHT: f64 = WINDOW_SIZE + STATS_AREA_HEIGHT; // New total window height

#[derive(Default)]
struct SimulationStats {
    total_births: usize,
    total_deaths: usize,
    max_population: usize,
    food_eaten: usize,
    energy_history: Vec<f32>,
    population_history: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BeingType {
    Herbivore,
    Carnivore,
    Omnivore,
}

#[derive(Clone, PartialEq)]
struct Genetics {
    speed: f32,
    size: f32,
    reproduction_rate: f32,
    perception: f32,
}

impl Genetics {
    fn new_random(being_type: BeingType) -> Self {
        let mut rng = rand::rng();
        let (speed_range, perception_range) = match being_type {
            BeingType::Carnivore => (1.0..3.0, 15.0..30.0),
            BeingType::Omnivore => (0.8..2.5, 12.0..25.0),
            BeingType::Herbivore => (0.5..2.0, 5.0..20.0),
        };
        
        Genetics {
            speed: rng.random_range(speed_range),
            size: rng.random_range(0.8..1.2),
            reproduction_rate: rng.random_range(0.5..1.5),
            perception: rng.random_range(perception_range),
        }
    }

    fn mutate(&self) -> Self {
        let mut rng = rand::rng();
        Genetics {
            speed: (self.speed * rng.random_range(0.9..1.1)).clamp(0.5, 3.0),
            size: (self.size * rng.random_range(0.9..1.1)).clamp(0.5, 2.0),
            reproduction_rate: (self.reproduction_rate * rng.random_range(0.9..1.1)).clamp(0.1, 2.0),
            perception: (self.perception * rng.random_range(0.9..1.1)).clamp(2.0, 30.0),
        }
    }
}

#[derive(Clone, PartialEq)]
struct Being {
    x: f64,
    y: f64,
    color: [f32; 4],
    energy: f32,
    being_type: BeingType,
    genetics: Genetics,
    age: u32,
    max_age: u32,
}

impl Being {
    fn new(x: f64, y: f64, being_type: BeingType) -> Self {
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

    fn size(&self) -> f64 {
        BASE_BEING_SIZE * self.genetics.size as f64
    }

    fn update(&mut self, beings: &[Being], foods: &[Food]) -> (Vec<usize>, Option<Being>) {
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
        self.energy -= ENERGY_DECAY * (self.genetics.size + self.genetics.speed);
        
        let perception_range = self.genetics.perception as f64;
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

    fn update_herbivore(
        &mut self,
        foods: &[Food],
        perception_range: f64,
        rng: &mut impl Rng,
        eaten_food_indices: &mut Vec<usize>,
    ) {
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

    fn update_carnivore(&mut self, beings: &[&Being], perception_range: f64, rng: &mut impl Rng) -> Option<Being> {
	if let Some(target) = beings.iter()
            .filter(|&&b| b.size() < self.size())
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
		let speed_boost = if distance < perception_range / 2.0 { 1.5 } else { 1.0 };
		self.x += dx / distance * self.genetics.speed as f64 * 2.5 * speed_boost;
		self.y += dy / distance * self.genetics.speed as f64 * 2.5 * speed_boost;
		
		if distance < self.size() / 2.0 + target.size() / 2.0 {
                    self.energy += target.energy * 0.95;
                    return Some((*target).clone());  
		}
            }
	}
	self.random_movement(rng);
	None
    }
    
    fn update_omnivore(
	&mut self,
	beings: &[&Being],
	foods: &[Food],
	perception_range: f64,
	rng: &mut impl Rng,
    ) -> Option<(Option<Being>, Vec<usize>)> {
	let mut eaten_food_indices = Vec::new();
	
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
    
    fn random_movement(&mut self, rng: &mut impl Rng) {
        self.x += rng.random_range(-1.0..1.0) * self.genetics.speed as f64;
        self.y += rng.random_range(-1.0..1.0) * self.genetics.speed as f64;
    }

    fn can_replicate(&self) -> bool {
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

    fn replicate(&mut self) -> Being {
        let mut child = self.clone();
        let mut rng = rand::rng();
        
        child.x += rng.random_range(-20.0..20.0);
        child.y += rng.random_range(-20.0..20.0);
        child.energy = self.energy * 0.5;
        child.genetics = self.genetics.mutate();
        child.age = 0;
        self.energy *= 0.5;
        
        child
    }

    fn draw(&self, transform: math::Matrix2d, g: &mut G2d) {
        let size = self.size();
        rectangle(
            self.color,
            [self.x, self.y, size, size],
            transform,
            g,
        );
    }
}

#[derive(Clone, PartialEq)]
struct Food {
    x: f64,
    y: f64,
    energy: f32,
}

impl Food {
    fn new() -> Self {
        let mut rng = rand::rng();
        Food {
            x: rng.random_range(0.0..WINDOW_SIZE),
            y: rng.random_range(0.0..WINDOW_SIZE),
            energy: rng.random_range(0.3..0.7),
        }
    }
    
    fn draw(&self, transform: math::Matrix2d, g: &mut G2d) {
        let size = 5.0;
        rectangle(
            [0.0, 1.0, 0.0, 1.0],
            [self.x, self.y, size, size],
            transform,
            g,
        );
    }
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new(
        "Parallel Virtual Ecosystem",
        [WINDOW_SIZE as u32, TOTAL_WINDOW_HEIGHT as u32], // Updated window height
    )
    .exit_on_esc(true)
    .build()
    .unwrap();
    
    let mut beings = vec![
        Being::new(WINDOW_SIZE / 3.0, WINDOW_SIZE / 3.0, BeingType::Herbivore),
        Being::new(WINDOW_SIZE / 4.0, WINDOW_SIZE / 4.0, BeingType::Herbivore),
        Being::new(WINDOW_SIZE * 2.0 / 3.0, WINDOW_SIZE / 3.0, BeingType::Carnivore),
        Being::new(WINDOW_SIZE * 3.0 / 4.0, WINDOW_SIZE / 4.0, BeingType::Carnivore),
        Being::new(WINDOW_SIZE / 2.0, WINDOW_SIZE * 2.0 / 3.0, BeingType::Omnivore),
    ];
    
    let foods = Arc::new(Mutex::new(Vec::<Food>::new()));
    let mut rng = rand::rng();
    
    // Load font
    let mut glyphs = {
        let font_path = std::path::Path::new("assets/FiraSans-Regular.ttf");
        if font_path.exists() {
            window.load_font(font_path).ok()
        } else {
            eprintln!("Could not load font file at {:?}", font_path);
            None
        }
    };
    
    let mut stats = SimulationStats {
        energy_history: Vec::with_capacity(1000),
        population_history: Vec::with_capacity(1000),
        ..Default::default()
    };
    
    while let Some(e) = window.next() {
        // Track population history
        stats.population_history.push(beings.len());
        if beings.len() > stats.max_population {
            stats.max_population = beings.len();
        }
        
        // Spawn food
        if foods.lock().unwrap().len() < MAX_FOOD && rng.random_range(0.0..1.0) < FOOD_SPAWN_RATE {
            foods.lock().unwrap().push(Food::new());
        }
        
        // Parallel being updates
        let beings_copy = beings.clone();
        let foods_copy = foods.lock().unwrap().clone();
        let foods_ref = Arc::clone(&foods);
        
        let updates: Vec<(Being, Vec<usize>, Option<Being>)> = beings.par_iter_mut()
            .map(|being| {
                let (eaten_food_indices, new_being) = being.update(&beings_copy, &foods_copy);
                (being.clone(), eaten_food_indices, new_being)
            })
            .collect();
        
        // Process updates and track statistics
        {
            let mut foods = foods_ref.lock().unwrap();
            for (_, eaten_food_indices, _) in updates.iter() {
                stats.food_eaten += eaten_food_indices.len();
                for &idx in eaten_food_indices.iter().rev() {
                    if idx < foods.len() {
                        foods.remove(idx);
                    }
                }
            }
        }
        
        // Track energy history
        if !beings.is_empty() {
            let avg_energy = beings.iter().map(|b| b.energy).sum::<f32>() / beings.len() as f32;
            stats.energy_history.push(avg_energy);
        }
        
        // Update beings and track births/deaths
        beings = updates.into_iter()
            .flat_map(|(being, _, new_being)| {
                if new_being.is_some() {
                    stats.total_births += 1;
                }
                let mut beings = Vec::new();
                beings.push(being);
                if let Some(b) = new_being {
                    beings.push(b);
                }
                beings
            })
            .filter(|b| {
                if b.energy <= 0.0 || b.age > b.max_age {
                    stats.total_deaths += 1;
                    false
                } else {
                    true
                }
            })
            .collect();
        
        // Enforce population limit
        if beings.len() > MAX_BEINGS {
            beings.truncate(MAX_BEINGS);
        }
        
        // Keep history buffers manageable
        if stats.population_history.len() > 1000 {
            stats.population_history.remove(0);
        }
        if stats.energy_history.len() > 1000 {
            stats.energy_history.remove(0);
        }
        
        // Draw everything
        window.draw_2d(&e, |c, g, device| {
            // Clear entire window
            clear([0.1, 0.1, 0.1, 1.0], g);
            
            // Draw stats area background
            rectangle(
                [0.2, 0.2, 0.2, 1.0], // Darker background for stats area
                [0.0, 0.0, WINDOW_SIZE, STATS_AREA_HEIGHT],
                c.transform,
                g,
            );
            
            // Draw stats text
            if let Some(ref mut glyphs) = glyphs {
                let stats_text = format!(
                    "Population: {} (H:{} C:{} O:{}) | Food: {} | Threads: {}",
                    beings.len(),
                    beings.iter().filter(|b| b.being_type == BeingType::Herbivore).count(),
                    beings.iter().filter(|b| b.being_type == BeingType::Carnivore).count(),
                    beings.iter().filter(|b| b.being_type == BeingType::Omnivore).count(),
                    foods.lock().unwrap().len(),
                    rayon::current_num_threads()
                );
                
                text::Text::new_color([1.0, 1.0, 1.0, 1.0], 20)
                    .draw(
                        &stats_text,
                        glyphs,
                        &c.draw_state,
                        c.transform.trans(10.0, 30.0),
                        g,
                    )
                    .unwrap();
                
                glyphs.factory.encoder.flush(device);
            }
            
            // Create transform for simulation area (offset by STATS_AREA_HEIGHT)
            let sim_transform = c.transform.trans(0.0, STATS_AREA_HEIGHT);
            
            // Draw foods in simulation area
            let foods = foods.lock().unwrap();
            for food in foods.iter() {
                food.draw(sim_transform, g);
            }
            
            // Draw beings in simulation area
            for being in &beings {
                being.draw(sim_transform, g);
            }
        });
    }
}
