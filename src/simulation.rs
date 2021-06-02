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

use crate::{
    keyed_set::prelude::*,
    physics::{self, prelude::*},
    window::DrawingContext,
    math,
};


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
    pub alive_time: f32,

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
    //  let h1 be the hunger after eating a food
    //  h1 = max( (h0 - hunger_reduction*h_max) / (1 + hunger_division),  0 )
    pub hunger_reduction: f32,
    pub hunger_division: f32,

    pub attack: f32,
    pub defence: f32,
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
    pub physics: physics::World,
}

impl Simulation {
    const SELECTION_LAYER: physics::Layer = physics::Layer::new(4);

    /// Create a simulation with a space of the given dimensions
    pub fn new(size: Vector2) -> Self {
        let mut collision_matrix = CollisionMatrix::new();
        collision_matrix.insert(Blob::LAYER, physics::LayerMask::new(vec![Food::LAYER, Blob::LAYER]));
        collision_matrix.insert(Food::LAYER, physics::LayerMask::empty());
        collision_matrix.insert(Blob::SIGHT_LAYER, physics::LayerMask::new(vec![Food::LAYER, Blob::LAYER]));
        collision_matrix.insert(Self::SELECTION_LAYER, physics::LayerMask::new(vec![Food::LAYER, Blob::LAYER]));
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

        let mut foods_to_remove = HashSet::new();
        let mut blobs_to_remove = HashMap::new();

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
                        let angle = math::unsigned_angle_vector2(dir, blob.direction).abs();
                        if angle > blob.pov { return None; }

                        let color = circle_object.color(self)?;
                        Some((circle_object, color, &circle.center))
                    })
                    .collect()
                );
            steps.insert(*key, blob.prepare_step(seen));
        }

        //  blobs eating
        for (_, blob) in &mut self.blobs {
            if let Some(touched) = collisions.get(&blob.circle) {
                for circle in touched {
                    if let Some(&CircleObject::Food(food)) = self.objects.get(circle) {
                        blob.eat(&mut self.foods, food);
                    }
                }
            }
        }

        //  blobs fighting
        let mut fights = HashSet::new();
        for (blob_key, blob) in &mut self.blobs {
            if let Some(touched) = collisions.get(&blob.circle) {
                for circle in touched {
                    if let Some(&CircleObject::Blob(other_blob_key)) = self.objects.get(circle) {
                        use std::cmp::{min, max};
                        let a = min(*blob_key, other_blob_key);
                        let b = max(*blob_key, other_blob_key);
                        fights.insert((a, b));
                    }
                }
            }
        }
        for (blob1_key, blob2_key) in fights {
            let blob1 = self.blobs.get(blob1_key).unwrap();
            let blob2 = self.blobs.get(blob2_key).unwrap();
            for &(attacker, _attacker_key, defender, defender_key) in &[(blob1, blob1_key, blob2, blob2_key), (blob2, blob2_key, blob1, blob1_key)] {
                if attacker.attack > defender.defence * (1. - defender.hunger / defender.max_hunger) {
                    blobs_to_remove.insert(defender_key, defender.pos);
                }
            }
        }

        //  step blobs
        let world = &mut self.physics;
        for (key, blob) in &mut self.blobs {
            blob.step(&steps[key], timestep, world, self.size);
        }

        //  blobs dying
        for (key, blob) in &self.blobs {
            if blob.hunger > blob.max_hunger {
                blobs_to_remove.insert(*key, blob.pos());
            }
        }
        
        //  remove
        for food in foods_to_remove {
            self.remove_food(food);
        }
        for (blob, pos) in blobs_to_remove {
            self.remove_blob(blob);
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
        attack: f32, defence: f32,
        hunger_reduction: f32, hunger_division: f32,
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
            alive_time: 0.,
            pos, radius, color,
            speed, rotation_speed,
            pov, sight_depth,
            favorite_color,
            color_attraction, color_repulsion,
            direction: Vector2::zero(),
            circle, sight_circle,
            max_hunger, hunger: 0.,
            attack, defence,
            hunger_reduction, hunger_division,
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

    pub fn set_blob_pos(&mut self, blob: Key<Blob>, pos: Vector2) {
        if let Some(blob) = self.blobs.get_mut(blob) {
            blob.set_pos(&mut self.physics, pos);
        }
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

    pub fn select(&mut self, pos: Vector2) -> (Vec<Key<Blob>>, Vec<Key<Food>>) {
        let mut foods = vec![];
        let mut blobs = vec![];
        let key = self.physics.circles.insert(Circle {
            center: pos, 
            radius: 0.01,
            layer: Self::SELECTION_LAYER,
        });
        let collisions = self.physics.collisions();
        self.physics.circles.remove(key);
        if let Some(collided) = collisions.get(&key) {
            for touched in collided {
                match self.objects.get(touched) {
                    Some(&CircleObject::Blob(blob)) => blobs.push(blob),
                    Some(&CircleObject::Food(food)) => foods.push(food),
                    _ => (),
                }
            }
            (blobs, foods)
        } else {
            (vec![], vec![])
        }
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

    fn feed(&mut self) { 
        //  h1 = max( (h0 - hunger_reduction*h_max) / (1 + hunger_division),  0 )
        self.hunger = f32::max(
            (self.hunger - self.hunger_reduction * self.max_hunger)
            /
            (1. + self.hunger_division),
            0.
        );
    }

    pub fn eat(&mut self, foods: &mut KeyedSet<Food>, food: Key<Food>) {
        self.feed();
        foods.remove(food);
    }

    pub fn draw(&self, draw: &mut DrawingContext) {

        const FONT_HEIGHT: i32 = 20;

        draw.draw_circle_v(self.pos, self.radius, self.fade_color(&self.color));
        
        if let Some(name) = &self.name {
            draw.draw_text(name,
                (self.pos().x - self.radius()) as i32,
                (self.pos().y - self.radius() - 2. * FONT_HEIGHT as f32) as i32,
                FONT_HEIGHT, self.fade_color(&self.favorite_color),
            );
        }

        //  draw time
        draw.draw_text(&format!("{:.1}", self.alive_time),
            (self.pos().x - self.radius()) as i32,
            (self.pos().y - self.radius() - FONT_HEIGHT as f32) as i32,
            FONT_HEIGHT, self.fade_color(&self.favorite_color),
        );

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
        if self.direction == Vector2::zero() {
            self.direction = random_vector2() * 2. - 1.;
        }
        else if let Some(target_direction) = step.target_direction {
            let t = self.rotation_speed * timestep;
            self.direction = math::slerp(self.direction, target_direction, t);
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

        //  do time
        self.alive_time += timestep;
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
