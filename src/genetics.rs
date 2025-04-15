use rand::Rng;
use super::BeingType;

#[derive(Clone, PartialEq)]
pub struct Genetics {
    pub speed: f32,
    pub size: f32,
    pub reproduction_rate: f32,
    pub perception: f32,
}

impl Genetics {
   pub fn new_random(being_type: BeingType) -> Self {
        let mut rng = rand::rng();
        let (speed_range, perception_range) = match being_type {
            BeingType::Carnivore => (2.0..4.0, 30.0..50.0), 
            BeingType::Omnivore => (0.8..2.5, 12.0..35.0),
            BeingType::Herbivore => (0.5..2.0, 6.0..25.0),
        };
        
        Genetics {
            speed: rng.random_range(speed_range),
            size: rng.random_range(0.8..1.2),
            reproduction_rate: rng.random_range(0.5..1.5),
            perception: rng.random_range(perception_range),
        }
    }

   pub fn mutate(&self) -> Self {
        let mut rng = rand::rng();
        Genetics {
            speed: (self.speed * rng.random_range(0.9..1.1)).clamp(0.5, 3.0),
            size: (self.size * rng.random_range(0.9..1.1)).clamp(0.5, 2.0),
            reproduction_rate: (self.reproduction_rate * rng.random_range(0.9..1.1)).clamp(0.1, 2.0),
            perception: (self.perception * rng.random_range(0.9..1.1)).clamp(2.0, 30.0),
        }
    }
}
