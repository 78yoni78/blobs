mod keyed_set;
mod window;
mod physics;
mod simulation;

use std::time;

use rand::random;

use raylib::prelude::*;

use crate::{
    window::prelude::*,
    simulation::prelude::*,
};

fn random_vector2() -> Vector2 { Vector2::new(random(), random()) }
fn random_color() -> Color { Color::new(random(), random(), random(), 255) }

fn add_random_blob(sim: &mut Simulation) -> keyed_set::Key<Blob> {
    sim.insert_blob(
        random_vector2() * sim.size(),
        20. * random::<f32>(),
        random_color(),
        120. * random::<f32>(),
        5. * random::<f32>(),
        180f32 * random::<f32>(),
        70f32 * random::<f32>(),
        random_color(),
        random(),
        random(),
        25. * random::<f32>()
    )
}

fn add_random_food(sim: &mut Simulation) -> keyed_set::Key<Food> {
    sim.insert_food(random_vector2() * sim.size())
}

fn main() {
    //  options
    let food_add_delay = time::Duration::from_secs_f32(0.01);
    let blob_add_delay = time::Duration::from_secs_f32(5.0);
    let start_blobs = 50;
    let start_foods = 1000;
    let simulation_config = Vector2::new(1040f32, 680f32);
    let window_config = WindowConfig {
        width: 1040,
        height: 680,
        title: "Blobs",
    }; 

    //  allocate resources
    let mut sim = Simulation::new(simulation_config);
    let mut window = Window::new(&window_config);
    let mut food_add_time = time::Instant::now(); 
    let mut blob_add_time = time::Instant::now(); 
    
    //  initialize simulation
    for _ in 0..start_blobs {
        add_random_blob(&mut sim);
    }
    //  initialize simulation
    for _ in 0..start_foods {
        add_random_food(&mut sim);
    }

    let mut last_frame_time = time::Instant::now();
    window.draw_loop(|mut draw| {
        //  record time and calculate delta
        let frame_time = time::Instant::now();
        let delta_time = (frame_time - last_frame_time).as_secs_f32();
        last_frame_time = frame_time;
        //  draw and simulate
        draw.clear_background(Color::WHITE);
        sim.draw(&mut draw);
        sim.step(delta_time);

        //  add blob
        if frame_time > blob_add_time {
            blob_add_time = frame_time + blob_add_delay;
            add_random_blob(&mut sim);            
        }
        //  add food
        if frame_time > food_add_time {
            food_add_time = frame_time + food_add_delay;
            add_random_food(&mut sim);
        }

        if draw.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
            add_random_blob(&mut sim);
        }
    });
}