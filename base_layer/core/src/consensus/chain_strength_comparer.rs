use crate::blocks::BlockHeader;
use std::cmp::Ordering;
use std::fmt::Debug;

pub trait ChainStrengthComparer: Debug {
    fn compare(&self, a: &BlockHeader, b: &BlockHeader) -> Ordering;
}

#[derive(Default, Debug)]
pub struct AccumulatedDifficultySquaredComparer {
}

impl ChainStrengthComparer for AccumulatedDifficultySquaredComparer{
    fn compare(&self, a: &BlockHeader, b: &BlockHeader) -> Ordering {
        let a_val = a.total_accumulated_difficulty_inclusive_squared();
        let b_val = b.total_accumulated_difficulty_inclusive_squared();
        if a_val < b_val {
            Ordering::Less
        }
            else {
                // f64's can never really be equal, so there is no `cmp` for f64
                // there are also weird NaN edge cases
                Ordering::Greater
            }
    }
}

#[derive(Debug)]
pub struct ThenComparer {
    before: Box<dyn ChainStrengthComparer + Send + Sync>,
    after: Box<dyn ChainStrengthComparer + Send + Sync>
}

impl ThenComparer {
    pub fn new(before : Box<dyn ChainStrengthComparer + Send + Sync>, after: Box<dyn ChainStrengthComparer + Send + Sync>) -> Self{
        ThenComparer{
            before, after
        }
    }
}

impl ChainStrengthComparer for ThenComparer {
    fn compare(&self, a: &BlockHeader, b: &BlockHeader) -> Ordering {
        match self.before.compare(a, b) {
            Ordering::Equal => self.after.compare(a, b),
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater
        }
    }
}





pub struct ChainStrengthComparerBuilder {
    target: Option<Box<dyn ChainStrengthComparer + Send + Sync>>
}

impl ChainStrengthComparerBuilder {
    pub fn new() -> ChainStrengthComparerBuilder {
        ChainStrengthComparerBuilder{
            target: None
        }
    }

    fn add_comparer_as_then(mut self, inner: Box<dyn ChainStrengthComparer + Send + Sync>) -> Self {
        self.target = match self.target {
            Some(t) => Some(Box::new(ThenComparer::new(t, inner))),
            None => Some(inner)
        };
        self
    }

    pub fn by_accumulated_difficulty( self) -> Self {
       self.add_comparer_as_then(Box::new(AccumulatedDifficultySquaredComparer::default()))
    }

    pub fn build( self) -> Box<dyn ChainStrengthComparer + Send + Sync> {
        self.target.unwrap()
    }

}

pub fn strongest_chain() -> ChainStrengthComparerBuilder {
    ChainStrengthComparerBuilder::new()
}
