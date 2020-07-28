mod geom;
mod types;

use elma::{lev::*, *};
use geom::*;
use lyon::{
    algorithms::hit_test::hit_test_path,
    math::point,
    path::{FillRule, Path},
};
use rand::{thread_rng, Rng};
use rand_distr::{Normal, Uniform};
use types::*;

/// Describes a position for a polygon on the level.
struct Placement {
    /// The polygon itself
    triangle: Triangle,
    /// The space no other polygon can be inside
    personal_space: f64,
    /// The apple for this polygon
    apple: Option<Point>,
}

fn get_allowed_area() -> Path {
    let mut builder = Path::builder();
    builder.move_to(point(33.9, 23.6));
    builder.line_to(point(12.7, 12.6));
    builder.line_to(point(14.0, -4.0));
    builder.line_to(point(26.0, -7.5));
    builder.line_to(point(54.8, -6.4));
    builder.line_to(point(62.6, -3.4));
    builder.line_to(point(67.4, 3.5));
    builder.line_to(point(70.7, 21.5));
    builder.line_to(point(39.9, 24.0));
    builder.line_to(point(37.2, 21.7));
    builder.build()
}

fn generate_random_placement() -> Placement {
    let mut rng = thread_rng();
    let p_vert_flip = 0.75;
    let p_apple = 0.5;
    let h_dist0 = Uniform::new(2.5, 5.0);
    let v_dist0 = Normal::new(0.0, 0.3).unwrap();
    let h_dist1 = Normal::new(1.0, 0.6).unwrap();
    let h_min = 2.5;
    let v_min = 1.5;
    let s_dist = Normal::new(1.0, 1.0).unwrap();
    let ps_dist = Uniform::new(1.0, 3.0);
    let ps_min = 1.0;
    let ps_max = 2.4;

    let a = [0.0, 0.0];

    let b = [
        a[0] + f64::max(rng.sample(h_dist0), h_min),
        a[1] + rng.sample(v_dist0),
    ];

    let width = f64::abs(b[0] - a[0]);
    let v_dist1 = Normal::new(width / 4.0, width / 4.0).unwrap();
    let will_vert_flip = rng.gen_bool(p_vert_flip);
    let c = [
        (b[0] + a[0]) / 2.0 + rng.sample(h_dist1),
        if will_vert_flip {
            a[1] - f64::max(rng.sample(v_dist1), v_min)
        } else {
            a[1] + f64::max(rng.sample(v_dist1), v_min)
        },
    ];

    let r_dist_rads = if will_vert_flip {
        Normal::new(0.0, 0.1).unwrap()
    } else {
        Normal::new(0.0, 0.2617994).unwrap()
    };

    let scale = f64::max(rng.sample(s_dist), 1.0);
    let shift_x = rng.gen_range(13.0, 61.0);
    let shift_y = rng.gen_range(-4.0, 21.0);

    let triangle = points_to_triangle(
        coords_to_point(&a),
        coords_to_point(&b),
        coords_to_point(&c),
    )
    .transform(
        &Transform::create_scale(scale, scale).post_translate(Vector::new(shift_x, shift_y)),
    );
    let rot_rads = rng.sample(r_dist_rads);
    let triangle = triangle.rotate_about(&triangle.center(), rot_rads);

    let personal_space = f64::min(ps_max, f64::max(ps_min, rng.sample(ps_dist)));

    let apple = if rng.gen_bool(p_apple) {
        Some(Point::new(
            if will_vert_flip {
                (triangle.b.x + triangle.a.x) / 2.0
            } else {
                triangle.c.x
            },
            if will_vert_flip {
                (triangle.b.y + triangle.a.y) / 2.0
            } else {
                triangle.c.y
            } + 0.8,
        ))
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

fn coords_to_point(arr: &[f64; 2]) -> Point {
    Point::new(arr[0], arr[1])
}

fn points_to_triangle(a: Point, b: Point, c: Point) -> Triangle {
    Triangle { a, b, c }
}

fn triangle_to_polygon(triangle: &Triangle) -> Polygon {
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

fn path_to_polygon(path: &Path) -> Polygon {
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

fn main() {
    let mut lev = Level::load("aht.lev").unwrap();
    let mut placements = Vec::<Placement>::new();
    let mut n_valid_placements = 0;
    let mut n_attempted_placements = 0;
    let allowed_area = get_allowed_area();

    while n_attempted_placements < 120000 {
        n_attempted_placements += 1;
        let new_placement = generate_random_placement();
        if placements.iter().all(|placement| {
            let dist = placement.triangle.distance_to(&new_placement.triangle);
            dist > placement.personal_space && dist > new_placement.personal_space
        }) && path_contains_triangle(&allowed_area, &new_placement.triangle)
        {
            n_valid_placements += 1;
            println!("{} / {}", n_valid_placements, n_attempted_placements);
            placements.push(new_placement);
        }
    }
    for placement in &placements {
        lev.polygons.push(triangle_to_polygon(&placement.triangle));
        if let Some(apple) = &placement.apple {
            if placements
                .iter()
                .all(|placement| !placement.triangle.contains_point(*apple))
            {
                lev.objects.push(apple_at(apple));
            }
        }
    }
    lev.save("ah.lev", Top10Save::Yes).unwrap();
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
