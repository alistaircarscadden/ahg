use crate::types::*;
use lyon::{algorithms::raycast::*, math::vector};
use std::cmp::Ordering;

pub trait ClosestPoint<T> {
    /// Find the closest point to `other` on `self`.
    fn closest_point_to(&self, other: &T) -> Point;
}

pub trait DistanceTo<T> {
    /// Find the shortest distance to `other` from `self`.
    fn distance_to(&self, other: &T) -> f64;
}

pub trait Center {
    fn center(&self) -> Point;
}

pub trait RotateAbout<T> {
    fn rotate_about(&self, anchor: &T, rads: f64) -> Self;
}

impl ClosestPoint<Point> for LineSegment {
    fn closest_point_to(&self, other: &Point) -> Point {
        let a2p = other.to_vector() - self.from.to_vector();
        let a2b = self.to_vector();
        let a2b2 = a2b.square_length();
        let a2p_dot_a2b = a2p.dot(a2b);
        let t = (a2p_dot_a2b / a2b2).min(1.0).max(0.0);

        self.sample(t)
    }
}

impl DistanceTo<Point> for LineSegment {
    fn distance_to(&self, other: &Point) -> f64 {
        let closest_point = self.closest_point_to(other);
        other.distance_to(closest_point)
    }
}

impl Center for Triangle {
    fn center(&self) -> Point {
        Point::new(
            (self.a.x + self.b.x + self.c.x) / 3.0,
            (self.a.y + self.b.y + self.c.y) / 3.0,
        )
    }
}

impl RotateAbout<Point> for Triangle {
    fn rotate_about(&self, anchor: &Point, rads: f64) -> Triangle {
        self.transform(
            &Transform::create_translation(-anchor.x, -anchor.y)
                .post_rotate(Angle::radians(rads))
                .post_translate(Vector::new(anchor.x, anchor.y)),
        )
    }
}

impl DistanceTo<Point> for Triangle {
    fn distance_to(&self, to: &Point) -> f64 {
        f64::min(
            self.ab().distance_to(to),
            f64::min(self.ac().distance_to(to), self.bc().distance_to(to)),
        )
    }
}

impl DistanceTo<Triangle> for Triangle {
    fn distance_to(&self, to: &Triangle) -> f64 {
        if self.intersects(to) {
            return 0.0;
        }

        *[
            self.distance_to(&to.a),
            self.distance_to(&to.b),
            self.distance_to(&to.c),
            to.distance_to(&self.a),
            to.distance_to(&self.b),
            to.distance_to(&self.c),
        ]
        .iter()
        .min_by(|a, b| f64::partial_cmp(a, b).unwrap_or(Ordering::Equal))
        .unwrap()
    }
}
