/// An array of 256 bits.
#[derive(Default, Copy, Clone, Debug)]
pub struct Bytes32 {
    pub bytes: [u8; 32],
}

macro_rules! from_primitive_impl {
    ($f:ident, $size:expr, $to:ident) => {
        impl From<[$f; $size]> for $to {
            fn from(a: [$f; $size]) -> Self {
                $to { bytes: a }
            }
        }
    };
}

macro_rules! from_primitive_ref_impl {
    ($f:ident, $size:expr, $to:ident) => {
        impl From<&[$f; $size]> for $to {
            fn from(a: &[$f; $size]) -> Self {
                $to { bytes: a.clone() }
            }
        }
    };
}

macro_rules! from_type_for_primitive_impl {
    ($f:ident, $to:ident, $size:expr) => {
        impl From<$f> for [$to; $size] {
            fn from(a: $f) -> Self {
                a.bytes
            }
        }
    };
}
from_primitive_impl!(u8, 32, Bytes32);

from_primitive_ref_impl!(u8, 32, Bytes32);

from_type_for_primitive_impl!(Bytes32, u8, 32);

#[cfg(test)]
mod tests {
    use super::Bytes32;

    macro_rules! test_conversions {
        ($type: ident, $size: expr, $test_name: ident) => {
            #[test]
            fn $test_name() {
                let raw = [1; $size];

                let uint = $type::from(raw);
                assert_eq!(uint.bytes[$size - 1], 1);
                let uint = $type::from(&raw);
                assert_eq!(uint.bytes[$size - 1], 1);

                let uint: $type = raw.into();
                assert_eq!(uint.bytes[$size - 1], 1);
                let uint: $type = (&raw).into();
                assert_eq!(uint.bytes[$size - 1], 1);

                let r: [u8; $size] = uint.into();
                assert_eq!(r[$size - 1], 1);
            }
        };
    }

    test_conversions!(Bytes32, 32, test_bytes32);
}
