use raylib::prelude::*;

pub use raylib::prelude::Vector3;

pub fn unsigned_angle_vector2(a: Vector2, b: Vector2) -> f32 {
    let mut angle = a.angle_to(b).to_degrees().abs();
    if angle > 180. { angle -= 180. }
    angle
}

pub fn slerp(start: Vector2, end: Vector2, time: f32) -> Vector2 {
    //  https://en.wikipedia.org/wiki/Slerp
    //  slerp(p0, p1, t) = sin((1-t)a) / sin a * p0 + sin ta / sin a * p1

    if (start - end).length_sqr() <= 0.01 { return start; }

    let p0 = start;
    let p1 = end;
    let t = time;
    let a = unsigned_angle_vector2(start, end).to_radians();
    let sa = a.sin();

    (p0 * (((1. - t) * a).sin() / sa) + p1 * ((t * a).sin() / sa)).normalized()
}