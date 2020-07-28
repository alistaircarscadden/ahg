mod geom;
mod types;

use crate::{geom::*, types::*};
use elma::{lev::*, *};
use lyon::{
    algorithms::hit_test::hit_test_path,
    math::point,
    path::{FillRule, Path},
};
use rand::{thread_rng, Rng};
use rand_distr::{Normal, Uniform};
use serde::{Deserialize, Serialize};
use std::{fs::File, time::Instant};

#[derive(Serialize, Deserialize)]
struct Config {
    template_lev: String,
    allowed_area: Path,
}

/// Describes a position for a polygon on the level.
struct Placement {
    /// The polygon itself
    triangle: Triangle,
    /// The space no other polygon can be inside
    personal_space: f64,
    /// The apple for this polygon
    apple: Option<Point>,
}

fn generate_random_placement(bounding_min: &Point, bounding_max: &Point) -> Placement {
    let mut rng = thread_rng();

    // Algorithm notes:
    // The triangle is defined by (a, b, c) where a is the left vertex,
    // b is the right vertex (roughly horizontal from a), and c is the
    // top vertex (roughly half way horizontal between a and b).
    // The triangle may be flipped, making c below the line ab.
    // a starts at (0, 0) but is shifted at the end of the algorithm with
    // the rest of the triangle.

    // Probability the polygon will be flipped (into 'flat top' orientation)
    let p_vert_flip = 0.75;
    // Probability an apple will be generated on this polygon
    let p_apple = 0.5;
    // Distribution of the horizontal delta of b from a
    let b_hor_dist = Uniform::new(2.5, 5.0);
    // Distribution of the vertical delta of b from a
    let b_ver_dist = Normal::new(0.0, 0.3).unwrap();
    // Distribution of the horizontal delta of c from a
    let c_hor_dist = Normal::new(0.0, 0.6).unwrap();
    // Minimum clamp of values sampled from b_hor_dist
    let h_min = 2.5;
    // Minimum clamp of values sampled from v_dist1
    let v_min = 1.5;
    // Distribution of triangle scaling
    let s_dist = Normal::new(1.0, 1.0).unwrap();
    // Distribution of personal space
    let ps_dist = Uniform::new(1.4, 3.8);
    // Clamp values for personal space
    let ps_min = 1.0;
    let ps_max = 2.4;

    let a = point(0.0, 0.0);

    let b = point(
        a.x + f64::max(rng.sample(b_hor_dist), h_min),
        a.y + rng.sample(b_ver_dist),
    );

    // Difference between b.x and a.x
    let triangle_width = f64::abs(b.x - a.x);
    // Distribution of the vertical delta of c from a
    let c_ver_dist = Normal::new(triangle_width / 4.0, triangle_width / 4.0).unwrap();
    // Whether or not the triangle will be flipped
    let will_vert_flip = rng.gen_bool(p_vert_flip);
    let c = point((b.x + a.x) / 2.0 + rng.sample(c_hor_dist), {
        let v_delta = f64::max(rng.sample(c_ver_dist), v_min);
        if will_vert_flip {
            a.y - v_delta
        } else {
            a.y + v_delta
        }
    });

    // Distribution of rotation, depends on whether or not its a flat top
    // or flat bottom triangle
    let r_dist_rads = if will_vert_flip {
        Normal::new(0.0, 0.1).unwrap()
    } else {
        Normal::new(0.0, 0.2617994).unwrap()
    };

    // Scaling factor for the entire triangle
    let scale = f64::max(rng.sample(s_dist), 1.0);

    // Shifting for the entire triangle
    let shift = Vector::new(
        rng.gen_range(bounding_min.x, bounding_max.x),
        rng.gen_range(bounding_min.y, bounding_max.y),
    );
    let rot_rads = rng.sample(r_dist_rads);
    let triangle = Triangle { a, b, c }
        .transform(&Transform::create_scale(scale, scale).post_translate(shift));
    let triangle = triangle.rotate_about(&triangle.center(), rot_rads);

    let personal_space = f64::min(ps_max, f64::max(ps_min, rng.sample(ps_dist)));

    let apple = if rng.gen_bool(p_apple) {
        let vert_shift_apple = 0.8;
        Some(if will_vert_flip {
            Point::new(
                (triangle.b.x + triangle.a.x) / 2.0,
                (triangle.b.y + triangle.a.y) / 2.0 + vert_shift_apple,
            )
        } else {
            Point::new(triangle.c.x, triangle.c.y + vert_shift_apple)
        })
    } else {
        None
    };

    Placement {
        triangle,
        personal_space,
        apple,
    }
}

fn apple_at(position: &Point) -> Object {
    let mut object = Object::new();
    object.object_type = ObjectType::Apple {
        gravity: GravityDirection::None,
        animation: 0,
    };
    object.position = Position::new(position.x, position.y);
    object
}

fn triangle_to_elma_polygon(triangle: &Triangle) -> Polygon {
    let mut polygon = Polygon::new();
    polygon
        .vertices
        .push(Position::new(triangle.a.x, triangle.a.y));
    polygon
        .vertices
        .push(Position::new(triangle.b.x, triangle.b.y));
    polygon
        .vertices
        .push(Position::new(triangle.c.x, triangle.c.y));
    polygon
}

fn path_to_elma_polygon(path: &Path) -> Polygon {
    use lyon::path::Event;

    let mut polygon = Polygon::new();
    for pt in path {
        match pt {
            Event::Begin { at: pos } => polygon
                .vertices
                .push(Position::new(pos.x as f64, pos.y as f64)),
            Event::Line { from: _, to: pos } => polygon
                .vertices
                .push(Position::new(pos.x as f64, pos.y as f64)),
            _ => (),
        }
    }
    polygon
}

fn path_contains_triangle(path: &Path, triangle: &Triangle) -> bool {
    type Pointf32 = euclid::Point2D<f32, euclid::UnknownUnit>;
    let tolerance = 0.5;
    let a = Pointf32::new(triangle.a.x as f32, triangle.a.y as f32);
    let b = Pointf32::new(triangle.b.x as f32, triangle.b.y as f32);
    let c = Pointf32::new(triangle.c.x as f32, triangle.c.y as f32);
    hit_test_path(&a, path.iter(), FillRule::NonZero, tolerance)
        && hit_test_path(&b, path.iter(), FillRule::NonZero, tolerance)
        && hit_test_path(&c, path.iter(), FillRule::NonZero, tolerance)
}

fn path_bounding_box(path: &Path) -> Rect {
    let rectf32 = lyon::algorithms::aabb::bounding_rect(path.iter());
    Rect::new(
        Point::new(rectf32.origin.x as f64, rectf32.origin.y as f64),
        Size::new(rectf32.size.width as f64, rectf32.size.height as f64),
    )
}

fn main() {
    let config: Config = {
        let path = if let Some(path) = std::env::args().nth(1) {
            println!("Using config: {} instead of default", &path);
            path
        } else {
            String::from("config_default.json")
        };
        let file = File::open(&path).unwrap();
        serde_json::from_reader(file).unwrap()
    };
    let mut lev = Level::load(&config.template_lev).unwrap();
    let mut placements = Vec::<Placement>::new();
    let mut n_valid_placements = 0;
    let mut n_attempted_placements = 0;
    let (bounding_min, bounding_max) = {
        let bounding_area = path_bounding_box(&config.allowed_area);
        (
            Point::new(bounding_area.origin.x, bounding_area.origin.y),
            Point::new(
                bounding_area.origin.x + bounding_area.size.width,
                bounding_area.origin.y + bounding_area.size.height,
            ),
        )
    };

    let start = Instant::now();

    while n_attempted_placements < 120_000 {
        n_attempted_placements += 1;
        let new_placement = generate_random_placement(&bounding_min, &bounding_max);
        if placements.iter().all(|placement| {
            let dist = placement.triangle.distance_to(&new_placement.triangle);
            dist > placement.personal_space && dist > new_placement.personal_space
        }) && path_contains_triangle(&config.allowed_area, &new_placement.triangle)
        {
            n_valid_placements += 1;
            println!("Elapsed: {:?} (placed: {}, attempts: {})", start.elapsed(), n_valid_placements, n_attempted_placements);
            placements.push(new_placement);
        }
    }
    println!("Elapsed: {:?} (Triangle generation complete)", start.elapsed());
    for placement in &placements {
        lev.polygons.push(triangle_to_elma_polygon(&placement.triangle));
        if let Some(apple) = &placement.apple {
            if placements
                .iter()
                .all(|placement| !placement.triangle.contains_point(*apple))
            {
                lev.objects.push(apple_at(apple));
            }
        }
    }
    println!("Apple validation: {:?}", start.elapsed());
    lev.title.push_str("auto lev");
    lev.save("out.lev", Top10Save::Yes).unwrap();
}

#[test]
fn closest_point() {
    let line = LineSegment {
        from: Point::new(0.0, 0.0),
        to: Point::new(1.0, 1.0),
    };
    assert_eq!(
        line.closest_point_to(&Point::new(0.0, 0.0)),
        Point::new(0.0, 0.0)
    );
    assert_eq!(
        line.closest_point_to(&Point::new(-1.0, 0.0)),
        Point::new(0.0, 0.0)
    );
    assert_eq!(
        line.closest_point_to(&Point::new(2.0, 2.0)),
        Point::new(1.0, 1.0)
    );
    assert!(
        line.closest_point_to(&Point::new(0.9, 0.9))
            .distance_to(Point::new(0.9, 0.9))
            < 0.01
    );
    assert!(
        line.closest_point_to(&Point::new(0.0, 1.0))
            .distance_to(Point::new(0.5, 0.5))
            < 0.01
    );
}
