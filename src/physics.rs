use std::collections::HashMap;

use raylib::prelude::*;

use crate::keyed_set::prelude::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Layer(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerMask(u32);

impl Layer {
    pub const fn new(num: u8) -> Self {
        let bits: u32 = 1u32 << num;
        Self(bits)
    }
}

impl LayerMask {
    pub const fn empty() -> Self { Self(0) }
    
    pub fn add(&mut self, Layer(bits): Layer) {
        self.0 |= bits;
    }
    
    pub fn new<I: IntoIterator<Item=Layer>>(i: I) -> Self {
        let mut ret = Self::empty();
        for l in i {
            ret.add(l);
        }
        ret
    }

    pub fn contains(&self, layer: &Layer) -> bool {
        (self.0 & layer.0) != 0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub center: Vector2,
    pub radius: f32,
    pub layer: Layer,
}

pub type CircleCollisions = HashMap<Key<Circle>, Vec<Key<Circle>>>;

pub type CollisionMatrix = HashMap<Layer, LayerMask>;

pub struct World {
    pub circles: KeyedSet<Circle>,    
    collision_matrix: CollisionMatrix,
}


impl Circle {
    pub fn intersects_x_axis(&self, other: &Self) -> bool {
        (other.center.x - self.center.x).abs() <= self.radius + other.radius
    }

    pub fn intersects(&self, other: &Self) -> bool {
        (other.center - self.center).length_sqr() <= (self.radius + other.radius) * (self.radius + other.radius)
    }
}

impl World {
    pub fn new(collision_matrix: CollisionMatrix) -> Self {
        Self { circles: KeyedSet::new(), collision_matrix }
    }

    fn layers_collide(collision_matrix: &CollisionMatrix, left: &Circle, right: &Circle) -> bool {
        match collision_matrix.get(&left.layer) {
            None => true,
            Some(layer_mask) => layer_mask.contains(&right.layer),
        }
    }

    fn collisions_naive<'a>(collision_matrix: &CollisionMatrix, circles: &Vec<(Key<Circle>, &'a Circle)>) -> CircleCollisions {
        let mut ret = CircleCollisions::new();
        for &(key, circle) in circles {
            let mut collided = vec![];
            for &(other_key, other_circle) in circles {
                if other_key != key 
                && circle.intersects(other_circle)
                && Self::layers_collide(collision_matrix, circle, other_circle) {
                    collided.push(other_key);
                }
            }
            if collided.len() > 0 { 
                ret.insert(key, collided);
            }
        }
        ret    
    }

    pub fn collisions(&self) -> CircleCollisions {
        //  use the sweep and prune algorithm

        //  edge case - no circles
        if self.circles.len() == 0 { return CircleCollisions::new() }

        //  sort by x axis
        let mut circles: Vec<(Key<Circle>, &Circle)> = self.circles
            .iter()
            .map(|tuple| (*tuple.0, tuple.1))
            .collect();
        //  this line will not work because the sort-key is a vector
        //circles.sort_by_key(|circle| circle.center.x);
        circles.sort_by(|a, b| a.1.center.x.partial_cmp(&b.1.center.x).unwrap());

        //  check for x-axis intersection between neighbors
        let mut x_axis_collisions = vec![];
        let mut active_interval = vec![circles[0]]; //   edge case where no 0th element is handled earlier
        for (key, circle) in circles.into_iter().skip(1) {
            if active_interval.iter().any(|other| other.1.intersects_x_axis(circle)) {
                active_interval.push((key, circle));
            } else {
                //  only report collisions between more than 1 circles
                if active_interval.len() > 1 {
                    x_axis_collisions.push(active_interval);
                }
                active_interval = vec![(key, circle)];
            }
        }
        x_axis_collisions.push(active_interval);
        
        let mut ret = HashMap::new();
        for interval in &x_axis_collisions {
            for (key, value) in Self::collisions_naive(&self.collision_matrix, interval) {
                ret.insert(key, value);
            }
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_2_body_collision() {
        let mut w = World::new(CollisionMatrix::new());
        let a = w.circles.insert(Circle { center: Vector2::new(5., 4.), radius: 2., layer: Layer::new(0) } );
        let b = w.circles.insert(Circle { center: Vector2::new(6., 6.), radius: 1., layer: Layer::new(0) } );
        
        assert_eq!(w.collisions(), [
            (a, vec![b]),
            (b, vec![a]),
        ].iter().cloned().collect());

        w.circles.get_mut(b).unwrap().center.x += 2.;
        
        assert_eq!(w.collisions(), [].iter().cloned().collect());
    }

    #[test]
    fn test_3_body_collision() {
        let mut w = World::new(CollisionMatrix::new());
        let a = w.circles.insert(Circle { center: Vector2::new(5., 4.), radius: 2., layer: Layer::new(0) } );
        let b = w.circles.insert(Circle { center: Vector2::new(7., 6.), radius: 1., layer: Layer::new(0) } );
        let c = w.circles.insert(Circle { center: Vector2::new(3., 7.), radius: 2., layer: Layer::new(0) } );
        
        assert_eq!(w.collisions(), [
            (a, vec![c, b]),
            (b, vec![a]),
            (c, vec![a]),
        ].iter().cloned().collect());

        w.circles.get_mut(c).unwrap().radius += 2.;
        
        assert_eq!(w.collisions(), [
            (a, vec![c, b]),
            (b, vec![c, a]),
            (c, vec![a, b]),
        ].iter().cloned().collect());
    }
}

pub mod prelude {
    pub use super::{
        Circle,
        CollisionMatrix,
    };
}
