use piston_window::*;
use rand::Rng;
use rayon::prelude::*;
use std::time::Instant;
use std::sync::{Arc, Mutex};

mod being;
mod food;
mod genetics;
mod simulation_stats;

use being::{Being, BeingType};
use food::Food;
use simulation_stats::SimulationStats;

const WINDOW_SIZE: f64 = 800.0;
const BASE_BEING_SIZE: f64 = 10.0;
const MAX_BEINGS: usize = 220;
const MAX_FOOD: usize = 790;
const FOOD_SPAWN_RATE: f64 = 0.99;
const ENERGY_DECAY: f32 = 0.0000015;
const STATS_AREA_HEIGHT: f64 = 50.0; // New constant for stats area height
const TOTAL_WINDOW_HEIGHT: f64 = WINDOW_SIZE + STATS_AREA_HEIGHT; // New total window height

fn main() {
    let mut window: PistonWindow = WindowSettings::new(
        "Parallel Virtual Ecosystem",
        [WINDOW_SIZE as u32, TOTAL_WINDOW_HEIGHT as u32], // Updated window height
    )
    .exit_on_esc(true)
    .build()
    .unwrap();

    // Initialize beings with different types
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

    let mut last_time = Instant::now();
    let mut fps = 0.0;
    
    while let Some(e) = window.next() {
	// Calculate FPS
        let now = Instant::now();
        let delta_time = now.duration_since(last_time).as_secs_f64();
        last_time = now;
        fps = 0.9 * fps + 0.1 * (1.0 / delta_time);
	
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
                // When processing eaten food:
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
		    "Pop: {}/{} | H:{} C:{} O:{} | Food: {} | Threads: {} | FPS: {:.1} ",
		    beings.len(),
		    MAX_BEINGS,
		    beings.iter().filter(|b| b.being_type == BeingType::Herbivore).count(),
		    beings.iter().filter(|b| b.being_type == BeingType::Carnivore).count(),
		    beings.iter().filter(|b| b.being_type == BeingType::Omnivore).count(),
		    foods.lock().unwrap().len(),
		    rayon::current_num_threads(),
		    fps
		);
		
		// White text on dark background
		text::Text::new_color([1.0, 1.0, 1.0, 1.0], 20)
		    .draw(
			&stats_text,
			glyphs,
			&c.draw_state,
			c.transform.trans(10.0, 30.0), // X,Y position
			g
		    )
		    .unwrap();
		
		// Important: Flush the glyphs
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
