use elma::{lev::*, *};
use euclid::{
    Angle as GenericAngle, Point2D, Rotation2D, Transform2D, Translation2D, UnknownUnit, Vector2D,
};
use lyon_geom::{LineSegment as GenericLineSegment, Triangle as GenericTriangle};
use rand::{thread_rng, Rng};
use rand_distr::{Normal, Uniform};

type Angle = GenericAngle<f64>;
type Vector = Vector2D<f64, UnknownUnit>;
type Triangle = GenericTriangle<f64>;
type LineSegment = GenericLineSegment<f64>;
type Point = Point2D<f64, UnknownUnit>;
type Rotation = Rotation2D<f64, UnknownUnit, UnknownUnit>;
type Translation = Translation2D<f64, UnknownUnit, UnknownUnit>;
type Transform = Transform2D<f64, UnknownUnit, UnknownUnit>;

/**
 * Describes a position for a polygon on the level.
 */
struct Placement {
    // The polygon itself
    triangle: Triangle,
    // The space no other polygon can be inside
    personal_space: f64,
    // The apple for this polygon
    apple: Option<Position<f64>>,
}

fn triangle_center(t: &Triangle) -> Point {
    Point::new((t.a.x + t.b.x + t.c.x) / 3.0, (t.a.y + t.b.y + t.c.y) / 3.0)
}

fn rotate_triangle(t: &Triangle, rads: f64) -> Triangle {
    let center = triangle_center(t);
    t.transform(
        &Transform::create_translation(-center.x, -center.y)
            .post_rotate(Angle::radians(rads))
            .post_translate(Vector::new(center.x, center.y)),
    )
}

fn dist_pt_lineseg(ls: &LineSegment, p: &Point) -> f64 {
    let l2 = ls.to_vector().square_length();

    if l2 == 0.0 {
        return p.distance_to(ls.from);
    }

    let t = f64::max(
        0.0,
        f64::min(
            1.0,
            (p.to_vector() - ls.from.to_vector()).dot(ls.to - ls.from) / l2,
        ),
    );

    let projection = ls.from + ((ls.to - ls.from) * t);

    return p.distance_to(projection);
}

fn dist_pt_triangle(t: &Triangle, p: &Point) -> f64 {
    f64::min(
        dist_pt_lineseg(&t.ab(), p),
        f64::min(dist_pt_lineseg(&t.ac(), p), dist_pt_lineseg(&t.bc(), p)),
    )
}

fn dist_triangle_triangle(t: &Triangle, v: &Triangle) -> f64 {
    if t.intersects(v) {
        return 0.0;
    }

    // im truly sorry to all my supporters
    f64::min(
        dist_pt_triangle(t, &v.a),
        f64::min(
            dist_pt_triangle(t, &v.b),
            f64::min(
                dist_pt_triangle(t, &v.c),
                f64::min(
                    dist_pt_triangle(v, &t.a),
                    f64::min(dist_pt_triangle(v, &t.b), dist_pt_triangle(v, &t.c)),
                ),
            ),
        ),
    )
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

    let triangle = triangle_from_points(&a, &b, &c).transform(
        &Transform::create_scale(scale, scale).post_translate(Vector::new(shift_x, shift_y)),
    );
    let rot_rads = rng.sample(r_dist_rads);
    let triangle = rotate_triangle(&triangle, rot_rads);

    let personal_space = f64::min(ps_max, f64::max(ps_min, rng.sample(ps_dist)));

    let apple = if rng.gen_bool(p_apple) {
        Some(Position::new(
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

fn apple_at(position: &Position<f64>) -> Object {
    let mut object = Object::new();
    object.object_type = ObjectType::Apple {
        gravity: GravityDirection::None,
        animation: 0,
    };
    object.position = position.clone();
    object
}

fn array_to_point2d(arr: &[f64; 2]) -> Point {
    Point2D::new(arr[0], arr[1])
}

fn triangle_from_points(a: &[f64; 2], b: &[f64; 2], c: &[f64; 2]) -> Triangle {
    Triangle {
        a: array_to_point2d(a),
        b: array_to_point2d(b),
        c: array_to_point2d(c),
    }
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

fn main() {
    let mut lev = Level::load("aht.lev").unwrap();
    let mut placements = Vec::<Placement>::new();
    let mut n_failed_placements = 0;
    let mut n_attempted_placements = 0;
    loop {
        n_attempted_placements += 1;
        let new_placement = generate_random_placement();
        let mut valid = true;
        for existing_placement in &placements {
            let dist =
                dist_triangle_triangle(&existing_placement.triangle, &new_placement.triangle);
            if dist < existing_placement.personal_space || dist < new_placement.personal_space {
                valid = false;
                break;
            }
        }
        if valid {
            println!("{} / {}", n_failed_placements, n_attempted_placements);
            placements.push(new_placement);
        } else {
            n_failed_placements += 1;
            if n_failed_placements > 120_000 {
                break;
            }
        }
    }
    for placement in &placements {
        lev.polygons.push(triangle_to_polygon(&placement.triangle));
        if let Some(apple) = &placement.apple {
            lev.objects.push(apple_at(apple));
        }
    }
    lev.save("ah.lev", Top10Save::Yes).unwrap();
}
