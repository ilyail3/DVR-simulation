use std::ops::Add;
use std::fmt::Display;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cost<W: Ord + Clone + Add<Output=W> + Display> {
    Zero,
    Value(W),
    Infinity
}

impl<W: Ord + Clone + Add<Output=W> + Display> PartialOrd for Cost<W> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else {
            match self {
                Cost::Infinity => Some(Ordering::Greater),
                Cost::Zero => Some(Ordering::Less),
                Cost::Value(w) => match other {
                    Cost::Infinity => Some(Ordering::Less),
                    Cost::Zero => Some(Ordering::Greater),
                    Cost::Value(w2) => Some(w.cmp(w2))
                }
            }
        }
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display> Ord for Cost<W> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display> Add for Cost<W> {
    type Output = Cost<W>;

    fn add(self, rhs: Self) -> Self::Output {
        match &self {
            Cost::Infinity => Cost::Infinity,
            Cost::Zero => rhs,
            Cost::Value(w) => match rhs {
                Cost::Infinity => Cost::Infinity,
                Cost::Zero => self,
                Cost::Value(w2) => Cost::Value(w.to_owned() + w2)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum DVValue<W: Ord + Clone + Add<Output=W> + Display> {
    Infinity,
    Distance(W, usize),
    DirectDistance(W),
    SameNode,
}

impl<W: Ord + Clone + Add<Output=W> + Display> Into<Cost<W>> for DVValue<W> {
    fn into(self) -> Cost<W> {
        match self {
            DVValue::Infinity => Cost::Infinity,
            DVValue::SameNode => Cost::Zero,
            DVValue::DirectDistance(w) => Cost::Value(w),
            DVValue::Distance(w,_) => Cost::Value(w)
        }
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display> Into<Cost<W>> for &DVValue<W> {
    fn into(self) -> Cost<W> {
        match self {
            DVValue::Infinity => Cost::Infinity,
            DVValue::SameNode => Cost::Zero,
            DVValue::DirectDistance(w) => Cost::Value(w.to_owned()),
            DVValue::Distance(w,_) => Cost::Value(w.to_owned())
        }
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display> PartialEq for DVValue<W> {
    fn eq(&self, other: &Self) -> bool {
        let cost_self:Cost<W> = self.into();
        let cost_other:Cost<W> = other.into();

        cost_self.eq(&cost_other)
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display> Cost<W> {
    pub fn to_dv_value(&self, through: usize, direct: bool) -> DVValue<W> {
        match self {
            Cost::Infinity => DVValue::Infinity,
            Cost::Zero => DVValue::SameNode,
            Cost::Value(w) =>
                if direct {
                    DVValue::DirectDistance(w.to_owned())
                } else {
                    DVValue::Distance(w.to_owned(), through)
                }
        }
    }
}