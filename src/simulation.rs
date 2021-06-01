//! Genetic simulation of natural selection.
//!
//! Module contains the simulation data structure which
//! contains the data needed to compute the genetic simulation
//! of blob that eat food.
//! 
//! The API is designed to enable modification and interaction 
//! with an executing simulation.
//!
//! # Example
//!
//! ```
//! use crate::simulation::prelude::*;
//! 
//! let mut sim = Simulation::new(SimulationConfig {
//!     size: Vector2::new(600., 800.)
//! });
//! 
//! sim.insert_blob(Blob::new());
//! ```

use std::collections::{HashMap, HashSet};

use rand::prelude::*;

use raylib::prelude::*;

use crate::{keyed_set::prelude::*, physics::{self, prelude::*}, window::DrawingContext};


/// Returns a vector2 with x in [0,1) and y in [0,1)
fn random_vector2() -> Vector2 { Vector2::new(random(), random()) }

/// Returns -1 for very different colors and 1 for same color
fn color_similarity(a: &Color, b: &Color) -> f32 {
    let a = a.color_to_hsv();
    let b = b.color_to_hsv();
    let angle_difference = {
        let v = (a.x - b.x).abs();
        if v <= 180. { v } else { 360. - v } 
    };
    let main_component = 1. - 2. * angle_difference / 180.;
    let ret = main_component * (1. - (a.y - b.y).abs()) * (1. - (a.z - b.z).abs());
    debug_assert!(-1. <= ret && ret <= 1.);
    ret
}

#[derive(Debug)]
pub struct Blob {
    pub name: Option<String>,

    pub speed: f32,
    pub rotation_speed: f32,
    radius: f32,
    pub color: Color,

    sight_depth: f32, 
    pub pov: f32, 
    pub favorite_color: Color, 
    pub color_attraction: f32,
    pub color_repulsion: f32,

    pos: Vector2,
    pub direction: Vector2,
    circle: Key<Circle>,
    sight_circle: Key<Circle>,

    pub hunger: f32,
    pub max_hunger: f32,
}

#[derive(Debug)]
pub struct Food {
    pos: Vector2,
    circle: Key<Circle>,
}

#[derive(Debug, Clone, Copy)]
pub enum CircleObject {
    Blob(Key<Blob>),
    Food(Key<Food>),
    BlobSight(Key<Blob>),
}

pub struct Simulation {
    size: Vector2,
    blobs: KeyedSet<Blob>,
    foods: KeyedSet<Food>,
    objects: HashMap<Key<Circle>, CircleObject>,
    physics: physics::World,
}

impl Simulation {
    /// Create a simulation with a space of the given dimensions
    pub fn new(size: Vector2) -> Self {
        let mut collision_matrix = CollisionMatrix::new();
        collision_matrix.insert(Blob::LAYER, physics::LayerMask::new(vec![Food::LAYER, Blob::LAYER]));
        collision_matrix.insert(Food::LAYER, physics::LayerMask::empty());
        collision_matrix.insert(Blob::SIGHT_LAYER, physics::LayerMask::new(vec![Food::LAYER, Blob::LAYER]));
        Self {
            size,
            blobs: KeyedSet::new(),
            foods: KeyedSet::new(),
            objects: HashMap::new(),
            physics: physics::World::new(collision_matrix),
        }
    }

    /// Returns the size of the simulation's space
    pub fn size(&self) -> Vector2 { self.size }

    /// Draw the simulation data onto a buffer.
    pub fn draw(&self, draw: &mut DrawingContext) {
        //  background
        draw.clear_background(Color::RAYWHITE);
        //  foods
        for (_, food) in &self.foods {
            food.draw(draw);
        }
        //  blobs
        for (_, blob) in &self.blobs {
            blob.draw(draw);
        }
    }

    /// Advance the simulation by a single iteration.
    ///
    /// The timestep is the fraction of seconds that has passed
    /// since the last step in the simulation.
    /// The step will be more accurate as the timestep is closer
    /// to 0.
    pub fn step(&mut self, timestep: f32) {
        debug_assert!(timestep >= 0.);

        //  run collision detection
        let collisions = self.physics.collisions();

        //  prepare blob steps
        let mut steps = HashMap::new();
        for (key, blob) in &self.blobs {
            let seen: Vec<(&CircleObject, &Color, &Vector2)> = 
                collisions.get(&blob.sight_circle)
                .map_or_else(|| vec![], |collided| 
                    collided.iter()
                    .filter_map(|&key| {
                        let circle = self.physics.circles.get(key).unwrap();
                        let circle_object = self.objects.get(&key).unwrap();
                        let dir = circle.center - blob.pos();
                        //  make sure object inside blob POV 
                        let mut angle = dir.angle_to(blob.direction).to_degrees().abs();
                        if angle > 180. { angle -= 180. }
                        if angle > blob.pov { return None; }

                        let color = circle_object.color(self)?;
                        Some((circle_object, color, &circle.center))
                    })
                    .collect()
                );
            steps.insert(*key, blob.prepare_step(seen));
        }

        //  remove foods
        let mut to_remove = HashSet::new();
        for (_, blob) in &mut self.blobs {
            if let Some(touched) = collisions.get(&blob.circle) {
                for circle in touched {
                    if let Some(&CircleObject::Food(food)) = self.objects.get(circle) {
                        to_remove.insert(food);
                        blob.hunger = 0.;
                    }
                }
            }
        }
        for food in to_remove {
            self.remove_food(food);
        }

        //  step blobs
        let world = &mut self.physics;
        for (key, blob) in &mut self.blobs {
            blob.step(&steps[key], timestep, world, self.size);
        }

        //  remove blobs
        let mut to_remove = HashMap::new();
        for (key, blob) in &self.blobs {
            if blob.hunger > blob.max_hunger {
                to_remove.insert(*key, blob.pos());
            }
        }
        for (key, pos) in to_remove {
            self.remove_blob(key);
            self.insert_food(pos);
        }
    }

    /// Put a blob in the simulation.
    pub fn insert_blob(&mut self, 
        pos: Vector2, radius: f32, color: Color,
        speed: f32, rotation_speed: f32,
        pov: f32, sight_depth: f32,
        favorite_color: Color,
        color_attraction: f32, color_repulsion: f32,
        max_hunger: f32,
    ) -> Key<Blob> {
        //  create blob
        let circle = self.physics.circles.insert(Circle {
            center: pos, radius: radius, layer: Blob::LAYER,
        });
        let sight_circle = self.physics.circles.insert(Circle {
            center: pos, radius: sight_depth, layer: Blob::SIGHT_LAYER,
        });
        let blob = Blob {
            name: None,
            pos, radius, color,
            speed, rotation_speed,
            pov, sight_depth,
            favorite_color,
            color_attraction, color_repulsion,
            direction: Vector2::zero(),
            circle, sight_circle,
            max_hunger, hunger: 0.,
        };
        //  insert blob data
        let key = self.blobs.insert(blob);
        self.objects.insert(circle, CircleObject::Blob(key));
        self.objects.insert(sight_circle, CircleObject::BlobSight(key));

        key
    }
    
    /// Get a blob from the simulation.
    pub fn get_blob(&self, blob: Key<Blob>) -> Option<&Blob> {
        self.blobs.get(blob)
    }
    /// Get a blob from the simulation.
    pub fn get_blob_mut(&mut self, blob: Key<Blob>) -> Option<&mut Blob> {
        self.blobs.get_mut(blob)
    }
    
    /// Remove a blob from the simulation.
    pub fn remove_blob(&mut self, blob: Key<Blob>) -> Option<Blob> {
        //  try remove blob
        let blob = self.blobs.remove(blob);
        //  remove blob objects
        if let Some(blob) = &blob {
            self.objects.remove(&blob.circle);
            self.objects.remove(&blob.sight_circle);
            self.physics.circles.remove(blob.circle);
            self.physics.circles.remove(blob.sight_circle);
        }

        blob
    }

    /// Put a food in the simulation.
    pub fn insert_food(&mut self, pos: Vector2) -> Key<Food> {
        //  create food
        let circle = self.physics.circles.insert(Circle {
            center: pos, radius: Food::RADIUS, layer: Food::LAYER,
        });
        let food = Food { pos, circle };
        //  insert data
        let key = self.foods.insert(food);
        self.objects.insert(circle, CircleObject::Food(key));
        
        key
    }
    
    /// Get a food from the simulation.
    pub fn get_food(&self, food: Key<Food>) -> Option<&Food> {
        self.foods.get(food)
    }
    /// Get a food from the simulation.
    pub fn get_food_mut(&mut self, food: Key<Food>) -> Option<&mut Food> {
        self.foods.get_mut(food)
    }
    
    /// Remove a food from the simulation.
    pub fn remove_food(&mut self, food: Key<Food>) -> Option<Food> {
        //  try remove food
        let food = self.foods.remove(food);
        //  remove food objects
        if let Some(food) = &food {
            self.objects.remove(&food.circle);
            self.physics.circles.remove(food.circle);
        }

        food
    }
}

pub struct BlobStep {
    target_direction: Option<Vector2>,
}

impl Blob {
    pub const LAYER: physics::Layer = physics::Layer::new(0);
    pub const SIGHT_LAYER: physics::Layer = physics::Layer::new(1);

    pub fn pos(&self) -> Vector2 { self.pos }

    pub fn set_pos(&mut self, world: &mut physics::World, value: Vector2) {
        self.pos = value;
        world.circles.get_mut(self.circle).unwrap().center = value;
        world.circles.get_mut(self.sight_circle).unwrap().center = value;
    }

    pub fn radius(&self) -> f32 { self.radius }

    pub fn set_radius(&mut self, world: &mut physics::World, value: f32) {
        self.radius = value;
        world.circles.get_mut(self.circle).unwrap().radius = value;    
    }

    pub fn direction(&self) -> Vector2 { self.direction }

    pub fn set_direction(&mut self, _world: &mut physics::World, value: Vector2) {
        self.direction = value;
    }

    pub fn sight_depth(&self) -> f32 { self.sight_depth }

    pub fn set_sight_depth(&mut self, world: &mut physics::World, value: f32) {
        self.sight_depth = value;
        world.circles.get_mut(self.sight_circle).unwrap().radius = value;
    }

    fn fade_color(&self, color: &Color) -> Color {
        color.fade(1. - self.hunger / self.max_hunger)
    }

    pub fn draw(&self, draw: &mut DrawingContext) {

        draw.draw_circle_v(self.pos, self.radius, self.fade_color(&self.color));
        
        if let Some(name) = &self.name {
            draw.draw_text(name,
                (self.pos().x - self.radius()) as i32,
                (self.pos().y - self.radius() - 20.) as i32,
                20, self.fade_color(&self.favorite_color),
            );
        }

        // //  sight drawing
        // let angle = self.direction.x.atan2(self.direction.y).to_degrees();
        // draw.draw_circle_sector_lines(
        //     self.pos,                       //  start
        //     self.sight_depth,               //  radius
        //     (angle - self.pov / 2.) as i32, //  start_angle
        //     (angle + self.pov / 2.) as i32, //  end_angle
        //     25,                             //  segments
        //     self.favorite_color,            //  color
        // );
        // draw.draw_line_v(self.pos, self.pos + self.direction * 3. * self.speed, self.favorite_color);
    }

    pub fn prepare_step<'a, I>(&self, seen: I) -> BlobStep
    where I: std::iter::IntoIterator<Item=(&'a CircleObject, &'a Color, &'a Vector2)> {

        let mut sum = Vector2::zero();
        let mut count = 0.;
        for (_, color, pos) in seen {

            let v = color_similarity(&self.favorite_color, color);
            let v = v * (if v > 0. { self.color_attraction } else { self.color_repulsion });
            
            if (*pos - self.pos).length_sqr() != 0. {
                let target_dir = (*pos - self.pos).normalized();
                sum += target_dir * v; 
                count += v.abs();
            }
        }
        
        let target_direction = if count == 0. || sum.length_sqr() == 0. {
            None
        } else {
            let d = (sum / count as f32).normalized();
            Some(d)
        };

        BlobStep { target_direction }
    }

    pub fn step(&mut self, step: &BlobStep, timestep: f32, physics_world: &mut physics::World, world_size: Vector2) {
        
        //  update direction
        if let Some(target_direction) = step.target_direction {
            let t = self.rotation_speed * timestep;
            self.direction = (target_direction * t + self.direction * (1. - t)).normalized();
        } else if self.direction == Vector2::zero() {
            self.direction = random_vector2() * 2. - 1.;
        }

        //  move position
        self.pos += self.direction * self.speed * timestep;
        physics_world.circles.get_mut(self.circle).unwrap().center = self.pos;
        physics_world.circles.get_mut(self.sight_circle).unwrap().center = self.pos;
        
        //  do hunger
        self.hunger += timestep;

        //  do border
        if self.pos().x > world_size.x {
            self.set_pos(physics_world, Vector2::new(world_size.x, self.pos().y));
            self.set_direction(physics_world, Vector2::new(-self.direction().x, self.direction().y));
        }
        if self.pos().y > world_size.y {
            self.set_pos(physics_world, Vector2::new(self.pos().x, world_size.y));
            self.set_direction(physics_world, Vector2::new(self.direction().x, -self.direction().y));
        }
        if self.pos().x < 0. {
            self.set_pos(physics_world, Vector2::new(0., self.pos().y));
            self.set_direction(physics_world, Vector2::new(-self.direction().x, self.direction().y));
        }
        if self.pos().y < 0. {
            self.set_pos(physics_world, Vector2::new(self.pos().x, 0.));
            self.set_direction(physics_world, Vector2::new(self.direction().x, -self.direction().y));
        }
    }
}

impl Food {
    pub const LAYER: physics::Layer = physics::Layer::new(2);
    pub const COLOR: Color = Color::GREEN;
    pub const RADIUS: f32 = 5.;

    pub fn pos(&self) -> Vector2 { self.pos }

    fn circle_mut<'a>(&self, physics_world: &'a mut physics::World) -> &'a mut Circle {
        physics_world.circles.get_mut(self.circle).unwrap()
    }

    pub fn set_pos(&mut self, physics_world: &mut physics::World, value: Vector2) {
        self.pos = value;
        self.circle_mut(physics_world).center = value;
    }

    pub fn draw(&self, draw: &mut DrawingContext) {
        draw.draw_circle_v(self.pos, Self::RADIUS, Self::COLOR);
    }
}

impl CircleObject {
    pub fn color<'a>(&self, sim: &'a Simulation) -> Option<&'a Color> {
        match *self {
            Self::Blob(blob) => sim.get_blob(blob).map(|x| &x.color),
            Self::Food(_) => Some(&Food::COLOR),
            Self::BlobSight(_) => None,
        }
    }
}

pub mod prelude {
    pub use super::*;
}
