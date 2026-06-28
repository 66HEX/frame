//! Lerp trait defines behaviour for interpolating between two values of the same type.

use std::{
    fmt::Debug,
    ops::{Add, Mul, Sub},
};

use crate::{
    Bounds, Corners, DevicePixels, Edges, Percentage, Pixels, Point, Radians, Rems, Rgba, Size,
    colors::Colors,
};

/// A trait for types that can be linearly interpolated.
pub trait Lerp<Output = Self>
where
    Self: Sized,
{
    /// Interpolates between `self` and `to` based on `delta`.
    fn lerp(&self, to: &Self, delta: f32) -> Output;
}

impl Lerp<f32> for bool {
    fn lerp(&self, to: &Self, delta: f32) -> f32 {
        lerp(*self as u8 as f32, *to as u8 as f32, delta)
    }
}

macro_rules! float_lerps {
    ( $( $ty:ty ),+ ) => {
        $(
            impl Lerp for $ty {
                fn lerp(&self, to: &Self, delta: f32) -> Self {
                    lerp(*self, *to, delta as $ty)
                }
            }
        )+
    };
}

float_lerps!(f32, f64);

macro_rules! int_lerps {
    ( $( $ty:ident as $ty_into:ident ),+ ) => {
        $(
            impl Lerp for $ty {
                fn lerp(&self, to: &Self, delta: f32) -> Self {
                    lerp(*self as $ty_into, *to as $ty_into, delta as $ty_into) as $ty
                }
            }
        )+
    };
}

int_lerps!(
    usize as f32,
    u8 as f32,
    u16 as f32,
    u32 as f32,
    u64 as f64,
    u128 as f64,
    isize as f32,
    i8 as f32,
    i16 as f32,
    i32 as f32,
    i64 as f64,
    i128 as f64
);

macro_rules! struct_lerps {
    ( $( $ty:ident $( < $gen:ident > )? { $( $n:ident ),+ } ),+ $(,)? ) => {
        $(
            impl$(<$gen: Lerp + Clone + Debug + Default + PartialEq>)? Lerp for $ty$(<$gen>)? {
                fn lerp(&self, to: &Self, delta: f32) -> Self {
                    $ty$(::<$gen>)? {
                        $(
                            $n: self.$n.lerp(&to.$n, delta)
                        ),+
                    }
                }
            }
        )+
    };
}

struct_lerps!(
    Point<T> { x, y },
    Size<T> { width, height },
    Edges<T> { top, right, bottom, left },
    Corners<T> { top_left, top_right, bottom_right, bottom_left },
    Bounds<T> { origin, size },
    Rgba { r, g, b, a },
    Colors { text, selected_text, background, disabled, selected, border, separator, container }
);

macro_rules! tuple_struct_lerps {
    ( $( $ty:ident ( $n:ty ) ),+ ) => {
        $(
            impl Lerp for $ty {
                fn lerp(&self, to: &Self, delta: f32) -> Self {
                    $ty(self.0.lerp(&to.0, delta))
                }
            }
        )+
    };
}

tuple_struct_lerps!(
    Radians(f32),
    Percentage(f32),
    DevicePixels(i32),
    Rems(f32),
    Pixels(f32)
);

fn lerp<T>(from: T, to: T, alpha: T) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<Output = T>,
{
    from + (to - from) * alpha
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use crate::px;

    #[test]
    fn test_f32_lerp() {
        let start = 0.0_f32;
        let end = 100.0_f32;

        assert_eq!(start.lerp(&end, 0.0), 0.0);
        assert_eq!(start.lerp(&end, 0.5), 50.0);
        assert_eq!(start.lerp(&end, 1.0), 100.0);
        assert_eq!(start.lerp(&end, 0.25), 25.0);
    }

    #[test]
    fn test_f64_lerp() {
        let start = 0.0_f64;
        let end = 100.0_f64;

        assert_eq!(start.lerp(&end, 0.0), 0.0);
        assert_eq!(start.lerp(&end, 0.5), 50.0);
        assert_eq!(start.lerp(&end, 1.0), 100.0);
    }

    #[test]
    fn test_f32_lerp_negative_values() {
        let start = -50.0_f32;
        let end = 50.0_f32;

        assert_eq!(start.lerp(&end, 0.0), -50.0);
        assert_eq!(start.lerp(&end, 0.5), 0.0);
        assert_eq!(start.lerp(&end, 1.0), 50.0);
    }

    #[test]
    fn test_integer_lerp() {
        // i32
        assert_eq!(0_i32.lerp(&100_i32, 0.0), 0);
        assert_eq!(0_i32.lerp(&100_i32, 0.5), 50);
        assert_eq!(0_i32.lerp(&100_i32, 1.0), 100);

        // u8
        assert_eq!(0_u8.lerp(&100_u8, 0.5), 50);

        // u16
        assert_eq!(0_u16.lerp(&1000_u16, 0.5), 500);

        // u32
        assert_eq!(0_u32.lerp(&10000_u32, 0.5), 5000);

        // u64
        assert_eq!(0_u64.lerp(&100000_u64, 0.5), 50000);

        // i64
        assert_eq!((-50000_i64).lerp(&50000_i64, 0.5), 0);

        // usize
        assert_eq!(0_usize.lerp(&100_usize, 0.5), 50);

        // isize
        assert_eq!((-100_isize).lerp(&100_isize, 0.5), 0);
    }

    #[test]
    fn test_point_lerp() {
        let start: Point<f32> = Point { x: 0.0, y: 0.0 };
        let end: Point<f32> = Point { x: 100.0, y: 200.0 };

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.x, 50.0);
        assert_eq!(mid.y, 100.0);

        let at_start = start.lerp(&end, 0.0);
        assert_eq!(at_start.x, 0.0);
        assert_eq!(at_start.y, 0.0);

        let at_end = start.lerp(&end, 1.0);
        assert_eq!(at_end.x, 100.0);
        assert_eq!(at_end.y, 200.0);
    }

    #[test]
    fn test_size_lerp() {
        let start: Size<f32> = Size {
            width: 10.0,
            height: 20.0,
        };
        let end: Size<f32> = Size {
            width: 110.0,
            height: 220.0,
        };

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.width, 60.0);
        assert_eq!(mid.height, 120.0);
    }

    #[test]
    fn test_edges_lerp() {
        let start: Edges<f32> = Edges {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        };
        let end: Edges<f32> = Edges {
            top: 10.0,
            right: 20.0,
            bottom: 30.0,
            left: 40.0,
        };

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.top, 5.0);
        assert_eq!(mid.right, 10.0);
        assert_eq!(mid.bottom, 15.0);
        assert_eq!(mid.left, 20.0);
    }

    #[test]
    fn test_corners_lerp() {
        let start: Corners<f32> = Corners {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: 0.0,
            bottom_left: 0.0,
        };
        let end: Corners<f32> = Corners {
            top_left: 4.0,
            top_right: 8.0,
            bottom_right: 12.0,
            bottom_left: 16.0,
        };

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.top_left, 2.0);
        assert_eq!(mid.top_right, 4.0);
        assert_eq!(mid.bottom_right, 6.0);
        assert_eq!(mid.bottom_left, 8.0);
    }

    #[test]
    fn test_bounds_lerp() {
        let start: Bounds<f32> = Bounds {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: 100.0,
                height: 100.0,
            },
        };
        let end: Bounds<f32> = Bounds {
            origin: Point { x: 50.0, y: 50.0 },
            size: Size {
                width: 200.0,
                height: 200.0,
            },
        };

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.origin.x, 25.0);
        assert_eq!(mid.origin.y, 25.0);
        assert_eq!(mid.size.width, 150.0);
        assert_eq!(mid.size.height, 150.0);
    }

    #[test]
    fn test_rgba_lerp() {
        let start = Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let end = Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.r, 0.5);
        assert_eq!(mid.g, 0.5);
        assert_eq!(mid.b, 0.5);
        assert_eq!(mid.a, 1.0);
    }

    #[test]
    fn test_rgba_lerp_with_alpha() {
        let start = Rgba {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };
        let end = Rgba {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.r, 0.5);
        assert_eq!(mid.g, 0.0);
        assert_eq!(mid.b, 0.5);
        assert_eq!(mid.a, 0.5);
    }

    #[test]
    fn test_pixels_lerp() {
        let start = px(0.0);
        let end = px(100.0);

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid, px(50.0));

        let at_start = start.lerp(&end, 0.0);
        assert_eq!(at_start, px(0.0));

        let at_end = start.lerp(&end, 1.0);
        assert_eq!(at_end, px(100.0));
    }

    #[test]
    fn test_rems_lerp() {
        let start = Rems(0.0);
        let end = Rems(2.0);

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.0, 1.0);
    }

    #[test]
    fn test_device_pixels_lerp() {
        let start = DevicePixels(0);
        let end = DevicePixels(100);

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.0, 50);
    }

    #[test]
    fn test_percentage_lerp() {
        let start = Percentage(0.0);
        let end = Percentage(100.0);

        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.0, 50.0);
    }

    #[test]
    fn test_radians_lerp() {
        let start = Radians(0.0);
        let end = Radians(std::f32::consts::PI);

        let mid = start.lerp(&end, 0.5);
        assert!((mid.0 - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    }
}
