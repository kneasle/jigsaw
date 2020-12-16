/// A newtype over [`usize`] that represents a stage.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Stage(usize);

macro_rules! gen_stage_consts {
    ( $( $name: ident => $val: literal ),* ) => {
        impl Stage {
            $( pub const $name: Stage = Stage($val); )*
        }
    };
}

gen_stage_consts!(
    SINGLES => 3,
    MINIMUS => 4,
    DOUBLES => 5,
    MINOR => 6,
    TRIPLES => 7,
    MAJOR => 8,
    CATERS => 9,
    ROYAL => 10,
    CINQUES => 11,
    MAXIMUS => 12,
    SEPTUPLES => 13,
    FOURTEEN => 14,
    SEXTUPLES => 15,
    SIXTEEN => 16
);

impl Stage {
    /// Returns this `Stage` as a [`usize`]
    pub fn as_usize(self) -> usize {
        self.0
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for Stage {
    fn from(n: usize) -> Self {
        Stage(n)
    }
}
