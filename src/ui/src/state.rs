use enum_map::{Enum, EnumMap};


pub trait ParameterKind: Enum + Copy {
    type Value: Copy + Default;
    type Spec: Copy;

    fn spec(self) -> Self::Spec;
}

pub type ParameterValues<P> = EnumMap<P, <P as ParameterKind>::Value>;

#[macro_export]
macro_rules! define_parameter_kind {
    (
        $d:tt;

        $visibility:vis struct $macro_name:ident ($value_type:ty) {
            $(
                $field_vis:vis $field:ident : $field_type:ty
            ),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        $visibility struct $macro_name {
            $(
                $field_vis $field: $field_type
            ),*
        }

        macro_rules! $macro_name {
            (
                $d enum_visibility:vis enum $d enum_name:ident {
                    $d(
                        $d variant:ident ($d($d param_name:ident: $d param:ty),* $d(,)?) {
                            $(
                                $field: $d $field:expr
                            ),* $d(,)?
                        }
                    ),*
                    $d(,)?
                }
            ) => {
                #[derive(Debug, Clone, Copy, PartialEq, Eq, enum_map::Enum)]
                $d enum_visibility enum $d enum_name {
                    $d(
                        $d variant($d($d param),*)
                    ),*
                }

                impl $d enum_name {
                    pub fn spec(self) -> $macro_name {
                        match self {
                            $d(
                                $d enum_name::$d variant ($d( $d param_name ),*) => $macro_name {
                                    $(
                                        $field: $d $field
                                    ),*
                                }
                            ),*
                        }
                    }
                }

                impl ParameterKind for $d enum_name {
                    type Value = $value_type;
                    type Spec = $macro_name;

                    fn spec(self) -> Self::Spec {
                        <$d enum_name>::spec(self)
                    }
                }
            };
        }
    }
}