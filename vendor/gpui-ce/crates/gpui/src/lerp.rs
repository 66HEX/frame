//! Lerp trait defines behaviour for interpolating between two values of the same type.
use crate::{
    AbsoluteLength, Background, Bounds, Corners, DefiniteLength, DevicePixels, Edges, Fill, Length,
    Percentage, Pixels, Point, Radians, Relative, Rems, RingColor, Size, colors::Colors,
};
use palette::{
    Hsla, IntoColor, Oklab, Oklaba,
    rgb::{Rgb, Rgba},
};
use std::{
    fmt::Debug,
    ops::{Add, Mul, Sub},
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
    Rgba { color, alpha },
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
    Relative(f32),
    Rems(f32),
    Pixels(f32)
);

macro_rules! new_constructor_lerps {
    ( $( $ty:ident $( < $gen:ident > )? ::new ( $( $n:ident ),+ ) ),+ $(,)? ) => {
        $(
            impl$(<$gen: Lerp + Clone + Debug + Default + PartialEq>)? Lerp for $ty$(<$gen>)? {
                fn lerp(&self, to: &Self, delta: f32) -> Self {
                    $ty$(::<$gen>)?::new(
                        $(
                            self.$n.lerp(&to.$n, delta)
                        ),+
                    )
                }
            }
        )+
    };
}

new_constructor_lerps!(
    Rgb::new(red, green, blue),
    Oklab::new(l, a, b),
    Oklaba::new(l, a, b, alpha)
);

impl Lerp for Hsla {
    fn lerp(&self, to: &Self, delta: f32) -> Self {
        if delta <= 0.0 {
            return *self;
        }
        if delta >= 1.0 {
            return *to;
        }

        let from: Rgba = (*self).into_color();
        let to: Rgba = (*to).into_color();
        from.lerp(&to, delta).into_color()
    }
}

impl Lerp for RingColor {
    fn lerp(&self, to: &Self, delta: f32) -> Self {
        match (*self, *to) {
            (Self::Color(from), Self::Color(to)) => Self::Color(from.lerp(&to, delta)),
            (_, to) if delta >= 1.0 => to,
            (from, _) => from,
        }
    }
}

impl Lerp for AbsoluteLength {
    fn lerp(&self, to: &Self, delta: f32) -> Self {
        match (*self, *to) {
            (Self::Pixels(from), Self::Pixels(to)) => Self::Pixels(from.lerp(&to, delta)),
            (Self::Rems(from), Self::Rems(to)) => Self::Rems(from.lerp(&to, delta)),
            (from, Self::Pixels(to)) if from.is_zero() => {
                Self::Pixels(Pixels::default().lerp(&to, delta))
            }
            (from, Self::Rems(to)) if from.is_zero() => {
                Self::Rems(Rems::default().lerp(&to, delta))
            }
            (Self::Pixels(from), to) if to.is_zero() => {
                Self::Pixels(from.lerp(&Pixels::default(), delta))
            }
            (Self::Rems(from), to) if to.is_zero() => {
                Self::Rems(from.lerp(&Rems::default(), delta))
            }
            _ if delta >= 1.0 => *to,
            _ => *self,
        }
    }
}

impl Lerp for DefiniteLength {
    fn lerp(&self, to: &Self, delta: f32) -> Self {
        match (*self, *to) {
            (Self::Absolute(from), Self::Absolute(to)) => Self::Absolute(from.lerp(&to, delta)),
            (Self::Relative(from), Self::Relative(to)) => Self::Relative(from.lerp(&to, delta)),
            _ if delta >= 1.0 => *to,
            _ => *self,
        }
    }
}

impl Lerp for Length {
    fn lerp(&self, to: &Self, delta: f32) -> Self {
        match (*self, *to) {
            (Self::Definite(from), Self::Definite(to)) => Self::Definite(from.lerp(&to, delta)),
            _ if delta >= 1.0 => *to,
            _ => *self,
        }
    }
}

impl Lerp for Background {
    fn lerp(&self, to: &Self, delta: f32) -> Self {
        if delta <= 0.0 {
            return *self;
        }
        if delta >= 1.0 {
            return *to;
        }

        match (self.as_solid(), to.as_solid()) {
            (Some(from), Some(to_color)) => {
                Background::from(from.lerp(&to_color, delta)).color_space(to.interpolation_space())
            }
            _ => *self,
        }
    }
}

impl Lerp for Fill {
    fn lerp(&self, to: &Self, delta: f32) -> Self {
        match (self, to) {
            (Self::Color(from), Self::Color(to)) => Self::Color(from.lerp(to, delta)),
        }
    }
}

fn lerp<T>(from: T, to: T, alpha: T) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<Output = T>,
{
    from + (to - from) * alpha
}
