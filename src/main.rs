mod keyed_set;
mod window;
mod physics;
mod simulation;
mod math;

use std::{
    time,
    io,
    fs,
    path,
};

use rand::{random, seq::SliceRandom};

use raylib::prelude::*;

use crate::{
    window::prelude::*,
    simulation::prelude::*,
};

fn random_vector2() -> Vector2 { Vector2::new(random(), random()) }
fn random_color() -> Color { Color::new(random(), random(), random(), 255) }

fn add_random_blob(sim: &mut Simulation, names: &mut Vec<String>) -> keyed_set::Key<Blob> {
    let key = sim.insert_blob(
        random_vector2() * sim.size(),
        20. * random::<f32>(),
        random_color(),
        120. * random::<f32>(),
        5. * random::<f32>(),
        180f32 * random::<f32>(),
        170f32 * random::<f32>(),
        random_color(),
        random(),
        random(),
        25. * random::<f32>(),
        random::<f32>(),
        2. * random::<f32>(),
        0.5 * random::<f32>(),
        random::<f32>(),
    );
    let name = names.choose(&mut rand::thread_rng()).unwrap().to_string();
    sim.get_blob_mut(key).unwrap().name = Some(name);
    key
}

fn add_random_food(sim: &mut Simulation) -> keyed_set::Key<Food> {
    sim.insert_food(random_vector2() * sim.size())
}

fn read_names<P: AsRef<path::Path> + ?Sized>(path: &P) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    Ok(content.split_whitespace().map(|x| x.to_string()).collect())
}  

fn main() {
    //  options
    let food_add_delay = time::Duration::from_secs_f32(0.2);
    let blob_add_delay = time::Duration::from_secs_f32(0.5);
    let start_blobs = 10;
    let start_foods = 100;
    let window_config = WindowConfig {
        width: 1300,
        height: 680,
        title: "Blobs",
    }; 

    //  allocate resources
    let mut window = Window::new(&window_config);
    let mut sim = Simulation::new(Vector2::new(window.width() as f32, window.height() as f32));
    let mut food_add_time = time::Instant::now(); 
    let mut blob_add_time = time::Instant::now(); 
    let mut names = read_names("names.txt").unwrap();
    
    //  initialize simulation
    for _ in 0..start_blobs {
        let blob_key = add_random_blob(&mut sim, &mut names);
    }
    //  initialize simulation
    for _ in 0..start_foods {
        add_random_food(&mut sim);
    }

    let mut last_frame_time = time::Instant::now();
    let mut prev_mouse_position = window.handle().get_mouse_position();
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
            let blob_key = add_random_blob(&mut sim, &mut names);
        }
        //  add food
        if frame_time > food_add_time {
            food_add_time = frame_time + food_add_delay;
            add_random_food(&mut sim);
        }

        if draw.is_key_down(KeyboardKey::KEY_SPACE) {
            add_random_blob(&mut sim, &mut names);
        }

        if draw.is_mouse_button_down(MouseButton::MOUSE_LEFT_BUTTON) {
            let (blobs, _foods) = sim.select(draw.get_mouse_position());
            for blob in blobs {
                sim.move_blob(blob, draw.get_mouse_position() - prev_mouse_position);
            }
        }

        prev_mouse_position = draw.get_mouse_position();
    });
}