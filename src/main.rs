use piston_window::*;
use rand::Rng;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

const WINDOW_SIZE: f64 = 800.0;
const BASE_BEING_SIZE: f64 = 10.0;
const MAX_BEINGS: usize = 260;
const MAX_FOOD: usize = 990;
const FOOD_SPAWN_RATE: f64 = 0.99;
const ENERGY_DECAY: f32 = 0.000001;

#[derive(Debug, Clone, Copy, PartialEq)]
enum BeingType {
    Herbivore,
    Carnivore,
    Omnivore,
}

#[derive(Clone)]
struct Genetics {
    speed: f32,
    size: f32,
    reproduction_rate: f32,
    perception: f32,
}

impl Genetics {
    fn new_random() -> Self {
        let mut rng = rand::thread_rng();
        Genetics {
            speed: rng.gen_range(0.5..2.0),
            size: rng.gen_range(0.8..1.2),
            reproduction_rate: rng.gen_range(0.5..1.5),
            perception: rng.gen_range(5.0..20.0),
        }
    }

    fn mutate(&self) -> Self {
        let mut rng = rand::thread_rng();
        Genetics {
            speed: (self.speed * rng.gen_range(0.9..1.1)).clamp(0.5, 3.0),
            size: (self.size * rng.gen_range(0.9..1.1)).clamp(0.5, 2.0),
            reproduction_rate: (self.reproduction_rate * rng.gen_range(0.9..1.1)).clamp(0.1, 2.0),
            perception: (self.perception * rng.gen_range(0.9..1.1)).clamp(2.0, 30.0),
        }
    }
}

#[derive(Clone)]
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
        let _rng = rand::thread_rng();
        let genetics = Genetics::new_random();
        
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

    fn update(&mut self, beings: &[Being], foods: &[Food]) -> (Vec<usize>, Vec<Being>) {
        let mut rng = rand::thread_rng();
        self.age += 1;
        self.energy -= ENERGY_DECAY * (self.genetics.size + self.genetics.speed);

        let perception_range = self.genetics.perception as f64;
        let mut eaten_food_indices = Vec::new();
        let mut new_beings = Vec::new();

        match self.being_type {
            BeingType::Herbivore => self.update_herbivore(foods, perception_range, &mut rng, &mut eaten_food_indices),
            BeingType::Carnivore => self.update_carnivore(beings, perception_range, &mut rng),
            BeingType::Omnivore => self.update_omnivore(beings, foods, perception_range, &mut rng, &mut eaten_food_indices),
        }

        self.x = self.x.max(0.0).min(WINDOW_SIZE - self.size());
        self.y = self.y.max(0.0).min(WINDOW_SIZE - self.size());

        if self.can_replicate() {
            new_beings.push(self.replicate());
        }

        (eaten_food_indices, new_beings)
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
                self.x += dx / distance * self.genetics.speed as f64 * 2.0;
                self.y += dy / distance * self.genetics.speed as f64 * 2.0;
                
                // Check if close enough to eat
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

    fn update_carnivore(&mut self, beings: &[Being], perception_range: f64, rng: &mut impl Rng) {
        if let Some(target) = beings.iter()
            .filter(|b| b.being_type != BeingType::Carnivore && b.size() < self.size())
            .min_by_key(|b| {
                let dx = b.x - self.x;
                let dy = b.y - self.y;
                ((dx * dx + dy * dy) * 1000.0) as i32
            }) 
        {
            let dx = target.x - self.x;
            let dy = target.y - self.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance < perception_range {
                self.x += dx / distance * self.genetics.speed as f64 * 2.0;
                self.y += dy / distance * self.genetics.speed as f64 * 2.0;
            } else {
                self.random_movement(rng);
            }
        } else {
            self.random_movement(rng);
        }
    }

    fn update_omnivore(
        &mut self,
        beings: &[Being],
        foods: &[Food],
        perception_range: f64,
        rng: &mut impl Rng,
        eaten_food_indices: &mut Vec<usize>,
    ) {
        if rng.gen_bool(0.5) {
            if let Some((idx, nearest_food)) = foods.iter().enumerate().min_by_key(|(_, f)| {
                let dx = f.x - self.x;
                let dy = f.y - self.y;
                ((dx * dx + dy * dy) * 1000.0) as i32
            }) {
                let dx = nearest_food.x - self.x;
                let dy = nearest_food.y - self.y;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance < perception_range {
                    self.x += dx / distance * self.genetics.speed as f64 * 2.0;
                    self.y += dy / distance * self.genetics.speed as f64 * 2.0;
                    
                    // Check if close enough to eat
                    if distance < self.size() / 2.0 + 2.5 {
                        eaten_food_indices.push(idx);
                        self.energy += nearest_food.energy;
                    }
                }
            }
        } else if let Some(target) = beings.iter()
            .filter(|b| b.size() < self.size())
            .min_by_key(|b| {
                let dx = b.x - self.x;
                let dy = b.y - self.y;
                ((dx * dx + dy * dy) * 1000.0) as i32
            }) 
        {
            let dx = target.x - self.x;
            let dy = target.y - self.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance < perception_range {
                self.x += dx / distance * self.genetics.speed as f64 * 2.0;
                self.y += dy / distance * self.genetics.speed as f64 * 2.0;
            }
        }
        
        self.random_movement(rng);
    }

    fn random_movement(&mut self, rng: &mut impl Rng) {
        self.x += rng.gen_range(-1.0..1.0) * self.genetics.speed as f64;
        self.y += rng.gen_range(-1.0..1.0) * self.genetics.speed as f64;
    }

    fn can_replicate(&self) -> bool {
        let mut rng = rand::thread_rng();
        self.energy > 0.7 && 
        rng.gen_range(0.0..1.0) < (0.001 * self.genetics.reproduction_rate) &&
        self.age > 100 &&
        self.age < self.max_age
    }

    fn replicate(&mut self) -> Being {
        let mut child = self.clone();
        let mut rng = rand::thread_rng();
        
        child.x += rng.gen_range(-20.0..20.0);
        child.y += rng.gen_range(-20.0..20.0);
        child.energy = self.energy * 0.5;
        child.genetics = self.genetics.mutate();
        child.age = 0;
        self.energy *= 0.5;
        
        child
    }

    fn draw(&self, c: Context, g: &mut G2d) {
        let size = self.size();
        rectangle(
            self.color,
            [self.x, self.y, size, size],
            c.transform,
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
        let mut rng = rand::thread_rng();
        Food {
            x: rng.gen_range(0.0..WINDOW_SIZE),
            y: rng.gen_range(0.0..WINDOW_SIZE),
            energy: rng.gen_range(0.3..0.7),
        }
    }
    
    fn draw(&self, c: Context, g: &mut G2d) {
        let size = 5.0;
        rectangle(
            [0.0, 1.0, 0.0, 1.0],
            [self.x, self.y, size, size],
            c.transform,
            g,
        );
    }
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new(
        "Parallel Virtual Ecosystem",
        [WINDOW_SIZE as u32, WINDOW_SIZE as u32],
    )
    .exit_on_esc(true)
    .build()
    .unwrap();

    let mut beings = vec![
        Being::new(WINDOW_SIZE / 3.0, WINDOW_SIZE / 3.0, BeingType::Herbivore),
        Being::new(WINDOW_SIZE * 2.0 / 3.0, WINDOW_SIZE / 3.0, BeingType::Carnivore),
        Being::new(WINDOW_SIZE / 2.0, WINDOW_SIZE * 2.0 / 3.0, BeingType::Omnivore),
    ];

    let foods = Arc::new(Mutex::new(Vec::new()));
    let mut rng = rand::thread_rng();
    let mut glyphs = window.load_font("assets/FiraSans-Regular.ttf").ok();

    while let Some(e) = window.next() {
        // Spawn food in parallel
        if foods.lock().unwrap().len() < MAX_FOOD && rng.gen_range(0.0..1.0) < FOOD_SPAWN_RATE {
            foods.lock().unwrap().push(Food::new());
        }

        // Parallel being updates
        let beings_copy = beings.clone();
        let foods_copy = foods.lock().unwrap().clone();
        let foods_ref = Arc::clone(&foods);

        let updates: Vec<_> = beings.par_iter_mut()
            .map(|being| {
                let (eaten_food_indices, new_beings) = being.update(&beings_copy, &foods_copy);
                (being.clone(), eaten_food_indices, new_beings)
            })
            .collect();

        // Process updates
        {
            let mut foods = foods_ref.lock().unwrap();
            // Remove eaten food by index (in reverse order to avoid shifting issues)
            for (_, eaten_food_indices, _) in updates.iter() {
                for &idx in eaten_food_indices.iter().rev() {
                    if idx < foods.len() {
                        foods.remove(idx);
                    }
                }
            }
        }
        
        // Add new beings
        for (_, _, new_beings) in updates {
            beings.extend(new_beings);
        }

        // Remove dead beings and enforce population limit
        beings.retain(|b| b.energy > 0.0 && b.age <= b.max_age);
        if beings.len() > MAX_BEINGS {
            beings.truncate(MAX_BEINGS);
        }

        // Draw everything
        window.draw_2d(&e, |c, g, device| {
            clear([0.1, 0.1, 0.1, 1.0], g);

            let foods = foods.lock().unwrap();
            for food in foods.iter() {
                food.draw(c, g);
            }

            for being in &beings {
                being.draw(c, g);
            }

            if let Some(ref mut glyphs) = glyphs {
                let stats = format!(
                    "Population: {} (H:{} C:{} O:{}) | Food: {} | Threads: {}",
                    beings.len(),
                    beings.iter().filter(|b| b.being_type == BeingType::Herbivore).count(),
                    beings.iter().filter(|b| b.being_type == BeingType::Carnivore).count(),
                    beings.iter().filter(|b| b.being_type == BeingType::Omnivore).count(),
                    foods.len(),
                    rayon::current_num_threads()
                );
                
                text::Text::new_color([1.0, 1.0, 1.0, 1.0], 20)
                    .draw(
                        &stats,
                        glyphs,
                        &c.draw_state,
                        c.transform.trans(10.0, 30.0),
                        g,
                    )
                    .unwrap();
                glyphs.factory.encoder.flush(device);
            }
        });
    }
}
