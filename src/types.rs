use euclid::UnknownUnit;

pub type Angle = euclid::Angle<f64>;
pub type Vector = euclid::Vector2D<f64, UnknownUnit>;
pub type Triangle = lyon::geom::Triangle<f64>;
pub type LineSegment = lyon::geom::LineSegment<f64>;
pub type Point = euclid::Point2D<f64, UnknownUnit>;
pub type Rotation = euclid::Rotation2D<f64, UnknownUnit, UnknownUnit>;
pub type Translation = euclid::Translation2D<f64, UnknownUnit, UnknownUnit>;
pub type Transform = euclid::Transform2D<f64, UnknownUnit, UnknownUnit>;
pub type Path = lyon::path::Path;
pub type QuadraticBezierSegment = lyon::geom::QuadraticBezierSegment<f64>;
pub type CubicBezierSegment = lyon::geom::CubicBezierSegment<f64>;
