use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::Add;

use crate::cost::{DVValue, Cost};
use std::slice::Iter;
use std::io::Write;
use std::fs::File;
use std::path::Path;

impl<W: Ord + Clone + Add<Output=W> + Display> DVValue<W> {
    pub fn write_html_long(&self, names: &BTreeMap<usize, String>) -> String {
        match self {
            DVValue::Infinity => String::from("&infin;"),
            DVValue::Distance(v, id) => format!(
                "{}({})",
                v,
                names.get(id).unwrap()
            ),
            DVValue::DirectDistance(v) => format!("{}", v),
            DVValue::SameNode => String::from("0")
        }
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display> Cost<W> {
    pub fn write_html(&self) -> String {
        match self {
            Cost::Infinity => String::from("&infin;"),
            Cost::Value(v) => format!("{}", v),
            Cost::Zero => String::from("0")
        }
    }
}



pub trait DistanceCalculationRepr : Clone {
    fn to_string(&self, names: &BTreeMap<usize, String>) -> String;
}

#[derive(Debug, Clone)]
pub enum DistanceCalculationElement {
    DirectDistance(usize, usize),
    DistanceVector(usize, usize),
}

impl DistanceCalculationRepr for DistanceCalculationElement {
    fn to_string(&self, names: &BTreeMap<usize, String>) -> String {
        match self {
            DistanceCalculationElement::DirectDistance(target, source) =>
                format!(
                    "C({},{})",
                    names.get(&source).unwrap(),
                    names.get(&target).unwrap()
                ),
            DistanceCalculationElement::DistanceVector(target, source) =>
                format!(
                    "d<sub>{}</sub>({})",
                    names.get(&source).unwrap(),
                    names.get(&target).unwrap()
                )
        }
    }
}

#[derive(Debug, Clone)]
pub struct DistanceCalculationTuple<W: Ord + Clone + Add<Output=W> + Display, R: DistanceCalculationRepr> {
    description: Vec<R>,
    result: Vec<Cost<W>>,
    through: usize,
    direct: bool
}

impl<W: Ord + Clone + Add<Output=W> + Display, R: DistanceCalculationRepr> DistanceCalculationTuple<W, R> {
    pub fn sum(&self) -> Cost<W> {
        let mut sum:Cost<W> = Cost::Zero;

        for item in &self.result {
            sum = sum + item.to_owned();
        }

        sum
    }

    pub fn sum_dv(&self) -> DVValue<W> {
        self.sum().to_dv_value(self.through, self.direct)
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display, R: DistanceCalculationRepr> PartialEq
    for DistanceCalculationTuple<W, R>{

    fn eq(&self, other: &Self) -> bool {
        self.sum().eq(&other.sum())
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display, R: DistanceCalculationRepr> Eq
    for DistanceCalculationTuple<W, R>{
}

impl<W: Ord + Clone + Add<Output=W> + Display, R: DistanceCalculationRepr> PartialOrd
    for DistanceCalculationTuple<W, R>{

    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.sum().partial_cmp(&other.sum())
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display, R: DistanceCalculationRepr> Ord
    for DistanceCalculationTuple<W, R> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sum().cmp(&other.sum())
    }
}

pub trait DistanceCalculationLine
    <W: Ord + Clone + Add<Output=W> + Display, R: DistanceCalculationRepr> {
    fn draw_direct(target: usize, src: usize) -> R;
    fn draw_distance(target: usize, src: usize) -> R;

    fn get_target(&self) -> usize;
    fn get_source(&self) -> usize;

    fn get_members(&self) -> Iter<DistanceCalculationTuple<W, R>>;

    fn render(&self, names: &BTreeMap<usize, String>) -> String {
        let mut result =
            Self::draw_distance(self.get_target(), self.get_source())
                .to_string(names);

        result += "=min(";

        let mut first = true;
        for desc in self.get_members() {
            if first {
                first = false;
            } else {
                result += ", ";
            }

            let mut first2 = true;
            for item in &desc.description {
                if first2 {
                    first2 = false;
                } else {
                    result += "+";
                }

                result += item.to_string(names).as_str();
            }
        }

        result += ")=min(";
        first = true;
        for desc in self.get_members() {
            if first {
                first = false;
            } else {
                result += ", ";
            }

            let mut first2 = true;
            for item in &desc.result {
                if first2 {
                    first2 = false;
                } else {
                    result += "+";
                }

                result += item.write_html().as_str();
            }
        }

        result += ")=";
        result += self.min_cost().write_html().as_str();

        result
    }

    fn add(&mut self, tuple: DistanceCalculationTuple<W,R>);

    fn add_indirect(
        &mut self,
        direct_target: usize,
        direct_src: usize,
        direct_cost: W,
        distance_target: usize,
        distance_src: usize,
        distance_cost: Cost<W>,
    ) {
        self.add(DistanceCalculationTuple {
            description: vec!(
                Self::draw_direct(direct_target, direct_src),
                Self::draw_distance(distance_target, distance_src)
            ),
            result: vec!(
                Cost::Value(direct_cost),
                distance_cost
            ),
            through: direct_target,
            direct: false
        })
    }

    fn add_direct(
        &mut self,
        direct_target: usize,
        direct_src: usize,
        direct_cost: W
    ) {
        self.add(DistanceCalculationTuple {
            description: vec!(Self::draw_direct(direct_target, direct_src)),
            result: vec!(Cost::Value(direct_cost)),
            through: direct_target,
            direct: true
        });
    }

    fn min_vector(&self) -> DVValue<W>;
    fn min_cost(&self) -> Cost<W>;
}

pub struct HtmlFormula<W: Ord + Clone + Add<Output=W> + Display>{
    target: usize,
    source: usize,
    members: Vec<DistanceCalculationTuple<W, DistanceCalculationElement>>
}

impl<W: Ord + Clone + Add<Output=W> + Display> DistanceCalculationLine<W, DistanceCalculationElement> for HtmlFormula<W>{
    fn draw_direct(target: usize, src: usize) -> DistanceCalculationElement {
        DistanceCalculationElement::DirectDistance(target, src)
    }

    fn draw_distance(target: usize, src: usize) -> DistanceCalculationElement {
        DistanceCalculationElement::DistanceVector(target, src)
    }

    fn get_target(&self) -> usize {
        self.target
    }

    fn get_source(&self) -> usize {
        self.source
    }

    fn get_members(&self) -> Iter<DistanceCalculationTuple<W, DistanceCalculationElement>> {
        self.members.iter()
    }

    fn add(&mut self, tuple: DistanceCalculationTuple<W, DistanceCalculationElement>) {
        self.members.push(tuple)
    }

    fn min_vector(&self) -> DVValue<W> {
        self.members
            .iter()
            .min()
            .unwrap()
            .sum_dv()
    }

    fn min_cost(&self) -> Cost<W> {
        self.members
            .iter()
            .min()
            .unwrap()
            .sum()
    }
}

impl<W: Ord + Clone + Add<Output=W> + Display> HtmlFormula<W>{
    pub fn new(target:usize, source:usize) -> Self {
        HtmlFormula{
            target,
            source,
            members: Vec::new()
        }
    }
}

pub struct HtmlFile{
    internal: Box<dyn Write>
}

impl Write for HtmlFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.internal.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.internal.flush()
    }
}

impl Drop for HtmlFile {
    fn drop(&mut self) {
        writeln!(self.internal, "</div>\n</body>\n</html>").unwrap();
    }
}

impl HtmlFile {
    pub fn new<P:AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = File::create(path)?;

        writeln!(file, "<!DOCTYPE html>")?;
        writeln!(file, "<html>\n<head>")?;
        writeln!(file, "<link rel=\"stylesheet\" href=\"styles.css\">")?;

        writeln!(file, "</head>\n<body>")?;
        writeln!(file, "<div class=\"wrapper\">")?;

        Ok(HtmlFile{ internal: Box::new(file) })
    }
}