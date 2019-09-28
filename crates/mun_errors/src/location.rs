use text_unit::{TextRange, TextUnit};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Location {
    Offset(TextUnit),
    Range(TextRange),
}

impl Into<Location> for TextUnit {
    fn into(self) -> Location {
        Location::Offset(self)
    }
}

impl Into<Location> for TextRange {
    fn into(self) -> Location {
        Location::Range(self)
    }
}

impl Location {
    pub fn offset(&self) -> TextUnit {
        match &self {
            Location::Offset(offset) => *offset,
            Location::Range(range) => range.start(),
        }
    }

    pub fn end_offset(&self) -> TextUnit {
        match &self {
            Location::Offset(offset) => *offset,
            Location::Range(range) => range.end(),
        }
    }

    pub fn add_offset(&self, plus_offset: TextUnit, minus_offset: TextUnit) -> Location {
        match &self {
            Location::Range(range) => Location::Range(range + plus_offset - minus_offset),
            Location::Offset(offset) => Location::Offset(offset + plus_offset - minus_offset),
        }
    }
}
