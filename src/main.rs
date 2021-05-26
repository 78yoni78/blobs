mod keyed_set;
mod window;
mod physics;

use std::{
    time,
    collections::HashMap,
};

use rand::prelude::*;

use raylib::prelude::*;

use crate::keyed_set::prelude::*;
use crate::window::prelude::*;


/// Returns -1 for very different colors and 1 for same color
pub fn color_dot(a: &Color, b: &Color) -> f32 {
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

pub struct Blob {
    pub speed: f32,
    pub rotation_speed: f32,
    pub rad: f32,
    pub color: Color,
    
    pub sight_depth: f32, 
    pub pov: f32, 
    pub favorite_color: Color, 
    pub color_attraction: f32,
    pub color_repulsion: f32,

    pub pos: Vector2,
    pub direction: Vector2,
    pub circle: physics::CircleKey,
    pub sight_circle: physics::CircleKey,
}

pub struct SimulationConfig {
    pub world_size: Vector2,
}

enum CircleObject {
    Blob(Key),
    BlobSight(Key),
    Food(Key),
}

impl CircleObject {
    pub fn color(&self, blobs: &KeyedSet<Blob>) -> Color {
        match self {
            &CircleObject::Blob(blob_key) => {
                blobs.get(blob_key).unwrap().color
            }
            CircleObject::Food(_) => Simulation::FOOD_COLOR,
            CircleObject::BlobSight(_) => panic!("Cannot get color of blob sight"),
        }
    }
}

pub struct Simulation {
    config: SimulationConfig,
    blobs: KeyedSet<Blob>,
    foods: KeyedSet<Vector2>,
    physics: physics::World,
    objects: HashMap<physics::CircleKey, CircleObject>,
}

impl Simulation {
    pub const BLOB_LAYER: physics::Layer = physics::Layer::new(0); 
    pub const FOOD_LAYER: physics::Layer = physics::Layer::new(1); 
    pub const SIGHT_LAYER: physics::Layer = physics::Layer::new(2); 
    pub const FOOD_COLOR: Color = Color::GREEN;

    pub fn new(config: SimulationConfig) -> Self {
        Self {
            config,
            blobs: KeyedSet::new(),
            foods: KeyedSet::new(),
            physics: physics::World::new([
                (Self::BLOB_LAYER, physics::LayerMask::new([Self::FOOD_LAYER].iter().cloned())),
                (Self::FOOD_LAYER, physics::LayerMask::new([Self::BLOB_LAYER].iter().cloned())),
                (Self::SIGHT_LAYER, physics::LayerMask::new([Self::BLOB_LAYER, Self::FOOD_LAYER].iter().cloned())),
            ].iter().cloned().collect()),
            objects: HashMap::new(),
        }
    }

    pub fn step(&mut self, draw: &mut DrawingContext, timestep: f32) {
        debug_assert!(timestep >= 0.);

        //  run collision detection
        let collisions = self.physics.collisions();

        //  draw foods
        for (_, food) in &self.foods {
            debug_assert!(!food.x.is_nan() && !food.y.is_nan());
            draw.draw_circle_v(food, 10f32, Self::FOOD_COLOR);
        }
        
        //  draw blobs
        for (_, blob) in &self.blobs {
            debug_assert!(!blob.pos.x.is_nan() && !blob.pos.y.is_nan());
            debug_assert!(!blob.direction.x.is_nan() && !blob.direction.y.is_nan());
            blob.draw(draw);
        }

        //  prepare blob steps
        let mut steps = HashMap::new();
        for (key, blob) in &self.blobs {
            let seen: Vec<(physics::CircleKey, Color)> = match collisions.get(&blob.sight_circle) {
                None => vec![],
                Some(collided) => collided
                    .iter()
                    .filter(|&&key| {
                        let circle = self.physics.get(key).unwrap();
                        let dir = circle.center - blob.pos;
                        let mut angle = dir.angle_to(blob.direction).to_degrees().abs();
                        if angle > 180. { angle -= 180. }
                        angle <= blob.pov
                    })
                    .map(|&key| {
                        (key, self.objects[&key].color(&self.blobs))
                    })
                    .collect(),
            };
            steps.insert(*key, blob.prepare_step(seen.as_slice(), &self.physics));
        }

        //  step blobs
        let world = &mut self.physics;
        for (key, blob) in &mut self.blobs {
            blob.step(&steps[key], timestep, world);
        }
    }

    pub fn add_random_blob(&mut self) {
        let blob = Blob::new_random(&self.config.world_size, &mut self.physics);
        //  copy the keys before moving blob into blobs
        let circle = blob.circle;   
        let sight = blob.sight_circle;   

        let blob_key = self.blobs.insert(blob);
        self.objects.insert(circle, CircleObject::Blob(blob_key));
        self.objects.insert(sight, CircleObject::BlobSight(blob_key));
    }

    pub fn add_random_food(&mut self) {
        let pos = Vector2 {
            x: random::<f32>() * self.config.world_size.x as f32,
            y: random::<f32>() * self.config.world_size.y as f32,
        };
        let key = self.foods.insert(pos);
        let circle_key = self.physics.insert(physics::Circle {
            center: pos,
            radius: 10f32,
            layer: Self::FOOD_LAYER,
        });
        self.objects.insert(circle_key, CircleObject::Food(key));
    }
}

#[derive(Debug)]
pub struct BlobStep {
    new_direction: Vector2,
}

impl Blob {
    pub fn new_random(world_size: &Vector2, world: &mut physics::World) -> Self {
        let rad = 20. * random::<f32>();
        let pos = Vector2 {
            x: random::<f32>() * world_size.x as f32,
            y: random::<f32>() * world_size.y as f32,
        };
        let sight_depth = 70f32 * random::<f32>();
        let circle = world.insert(physics::Circle { radius: rad, center: pos, layer: Simulation::BLOB_LAYER });
        let sight_circle = world.insert(physics::Circle { radius: sight_depth, center: pos, layer: Simulation::SIGHT_LAYER });
        Self {
            speed: 120. * random::<f32>(),
            rotation_speed: 5. * random::<f32>(),
            rad,
            color: Color::new(random(), random(), random(), 255),
            sight_depth, 
            pov: 180f32 * random::<f32>(), 
            
            favorite_color: Color::new(random(), random(), random(), 255),
            color_attraction: random::<f32>(),
            color_repulsion: random::<f32>(),

            pos,
            direction: Vector2 {
                x: 2f32 * random::<f32>() - 1f32,
                y: 2f32 * random::<f32>() - 1f32,
            }.normalized(),
            circle,
            sight_circle
        }
    }

    pub fn draw(&self, draw: &mut DrawingContext) {
        //  drawing
        draw.draw_circle_v(self.pos, self.rad, self.color);

        //  sight drawing
        let angle = self.direction.x.atan2(self.direction.y).to_degrees();
        draw.draw_circle_sector_lines(
            self.pos,                       //  start
            self.sight_depth,               //  radius
            (angle - self.pov / 2.) as i32, //  start_angle
            (angle + self.pov / 2.) as i32, //  end_angle
            25,                             //  segments
            self.favorite_color,            //  color
        );
        draw.draw_line_v(self.pos, self.pos + self.direction * 3. * self.speed, self.favorite_color);
    }

    pub fn prepare_step(&self, seen: &[(physics::CircleKey, Color)], world: &physics::World) -> BlobStep {

        let mut sum = Vector2::zero();
        let mut count = 0.;
        for (circle_key, color) in seen {
            let circle = world.get(*circle_key).unwrap();
            let v = color_dot(&self.favorite_color, color);
            let v = v * (if v > 0. { self.color_attraction } else { self.color_repulsion });
            
            if (circle.center - self.pos).length_sqr() != 0. {
                let target_dir = (circle.center - self.pos).normalized();
                sum += target_dir * v; 
                count += v.abs();
            }
        }
        
        let new_dir = if count == 0. || sum.length_sqr() == 0. { self.direction } else {
            (sum / count as f32).normalized()
        };

        BlobStep { new_direction: new_dir }
    }

    pub fn step<'a>(&mut self, step: &BlobStep, timestep: f32, world: &mut physics::World) {

        //  update direction
        let t = self.rotation_speed * timestep;
        self.direction = (step.new_direction * t + self.direction * (1. - t)).normalized();

        //  move position
        self.pos += self.direction * self.speed * timestep;
        world.get_mut(self.circle).unwrap().center = self.pos;
        world.get_mut(self.sight_circle).unwrap().center = self.pos;
    }
}

fn main() {
    //  options
    let start_blobs = 20;
    let start_foods = 200;
    let simulation_config = SimulationConfig {
        world_size: Vector2::new(1040f32, 680f32),
    };
    let window_config = WindowConfig {
        width: 1040,
        height: 680,
        title: "Blobs",
    }; 

    //  allocate resources
    let mut sim = Simulation::new(simulation_config);
    let mut window = Window::new(&window_config); 
    
    //  initialize simulation
    for _ in 0..start_blobs {
        sim.add_random_blob();
    }
    //  initialize simulation
    for _ in 0..start_foods {
        sim.add_random_food();
    }

    let mut last_frame_time = time::Instant::now();
    window.draw_loop(|mut draw| {
        //  record time and calculate delta
        let frame_time = time::Instant::now();
        let delta_time = (frame_time - last_frame_time).as_secs_f32();
        last_frame_time = frame_time;
        //  draw and simulate
        draw.clear_background(Color::WHITE);
        sim.step(&mut draw, delta_time);

        if draw.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
            sim.add_random_blob();
        }
    });
}