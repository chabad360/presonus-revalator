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

        $(
            $visibility:vis $name:ident ($value_type:ty) {
                $(
                    $field_vis:vis $field:ident : $field_type:ty
                ),* $(,)?
            }
        )*
    ) => {
        use paste::paste;

        $(
            paste! {
                #[derive(Debug, Clone, Copy, PartialEq)]
                $visibility struct $name {
                    $(
                        $field_vis $field: $field_type
                    ),*
                }

                macro_rules! [<$name:snake s>] {
                    (
                        $d(
                            $d variant:ident ($d($d param_name:ident: $d param:ty),* $d(,)?) {
                                $(
                                    $field: $d $field:expr
                                ),* $d(,)?
                            }
                        ),*
                        $d(,)?
                    ) => {
                        #[derive(Debug, Clone, Copy, PartialEq, Eq, enum_map::Enum)]
                        $visibility enum [<$name s>] {
                            $d(
                                $d variant($d($d param),*)
                            ),*
                        }

                        impl [<$name s>] {
                            pub fn spec(self) -> $name {
                                match self {
                                    $d(
                                        [<$name s>]::$d variant ($d( $d param_name ),*) => $name {
                                            $(
                                                $field: $d $field
                                            ),*
                                        }
                                    ),*
                                }
                            }
                        }

                        impl ParameterKind for [<$name s>] {
                            type Value = $value_type;
                            type Spec = $name;

                            fn spec(self) -> Self::Spec {
                                [<$name s>]::spec(self)
                            }
                        }
                    };
                }
            }
        )*
    }
}