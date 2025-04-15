#[derive(Default)]
pub struct SimulationStats {
    pub total_births: usize,
    pub total_deaths: usize,
    pub max_population: usize,
    pub food_eaten: usize,
    pub energy_history: Vec<f32>,
    pub population_history: Vec<usize>,
}
