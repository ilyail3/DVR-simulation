mod repr;
mod cost;

use std::collections::{HashMap, HashSet, BTreeMap};
use std::error::Error;
use std::fmt::Display;
use std::io::Write;
use std::ops::Add;
use crate::cost::DVValue;
use crate::repr::{HtmlFormula, DistanceCalculationLine, HtmlFile};
use std::path::Path;

#[derive(Debug, Clone)]
struct Neighbor<W: Ord + Clone + Add<Output=W> + Display> {
    index: usize,
    direct_cost: W,
    dv: Vec<DVValue<W>>
}

#[derive(Debug, Clone)]
struct Node<W: Ord + Clone + Add<Output=W> + Display> {
    name: String,
    dv: Vec<DVValue<W>>,
    neighbors: Vec<Neighbor<W>>,
    index: usize
}

#[derive(Debug)]
enum Operation<W: Ord + Clone + Add<Output=W> + Display> {
    ChangeWeight(usize, usize, W)
}

#[derive(Debug)]
struct World<W: Ord + Clone + Add<Output=W> + Display> {
    nodes: HashMap<String, Node<W>>,
    node_names: BTreeMap<usize, String>,
    generation: u32,
}

#[derive(Debug)]
enum NewState<W: Ord + Clone + Add<Output=W> + Display> {
    Changed(World<W>),
    NotChanged,
}

fn modify_dv<W: Ord + Clone + Add<Output=W> + Display>(
    original: &Vec<DVValue<W>>,
    node_b: usize,
    new_w: W,
) -> Vec<DVValue<W>> {
    let mut new_dv = Vec::new();
    let mut index: usize = 0;

    for v in original {
        if index == node_b {
            new_dv.push(DVValue::Distance(new_w.to_owned(), node_b));
        } else {
            new_dv.push(v.clone());
        }
        index += 1;
    }

    new_dv
}


impl<W: Ord + Clone + Add<Output=W> + Display> World<W> {
    pub fn new(node_names: Vec<&str>) -> World<W> {
        let mut result: HashMap<String, Node<W>> = HashMap::with_capacity(node_names.len());
        let mut node_names_out: BTreeMap<usize, String> = BTreeMap::new();
        let mut index: usize = 0;

        for name in &node_names {
            let mut dv_vector: Vec<DVValue<W>> = Vec::new();

            let mut sub_index: usize = 0;
            for _ in &node_names {
                if sub_index == index {
                    dv_vector.push(DVValue::SameNode)
                } else {
                    dv_vector.push(DVValue::Infinity)
                }

                sub_index += 1;
            }

            result.insert(name.to_string(), Node {
                name: name.to_string(),
                dv: dv_vector,
                neighbors: Vec::new(),
                index
            });

            node_names_out.insert(index, name.to_string());

            index += 1;
        }

        World { nodes: result, node_names: node_names_out, generation: 0 }
    }

    fn add_interface(&self, node_a: &str, node_b: &str, weight: W) -> Result<Operation<W>, Box<dyn Error>> {
        Ok(Operation::ChangeWeight(
            self.nodes
                .get(node_a)
                .map(|n| n.index)
                .ok_or("can't find node_a")?,
            self.nodes
                .get(node_b)
                .map(|n| n.index)
                .ok_or("can't find node_b")?,
            weight,
        ))
    }

    fn print_node<Writer: Write>(&self, writer: &mut Writer, node: &Node<W>, changed: Option<&Vec<DVValue<W>>>) -> Result<(), Box<dyn Error>> {
        writeln!(writer, "<table>\n\t<tr>")?;
        writeln!(writer, "\t\t<th>{}</th>", node.name)?;

        for node_name in self.node_names.values() {
            writeln!(writer, "\t\t<th>{}</th>", node_name)?;
        }

        writeln!(writer, "\t</tr>\n\t<tr>\n\t\t<th>{}</th>",node.name)?;
        // If there's a new dv, run the more complex algorithm
        if let Some(new_dv) = changed {
            for (index, new_value) in new_dv.iter().enumerate() {
                if new_value == node.dv.get(index).unwrap() {
                    writeln!(writer, "\t\t<td>{}</td>", new_value.write_html_long(&self.node_names))?;
                } else {
                    writeln!(
                        writer,
                        "\t\t<td>{}&#8594;{}</td>",
                        node.dv.get(index).unwrap().write_html_long(&self.node_names),
                        new_value.write_html_long(&self.node_names)
                    )?;
                }
            }
        } else {
            for new_value in &node.dv {
                writeln!(writer, "\t\t<td>{}</td>", new_value.write_html_long(&self.node_names))?;
            }
        }

        writeln!(writer, "\t</tr>")?;



        for neighbor in &node.neighbors {
            writeln!(
                writer,
                "\t<tr>\n\t\t<th>{}</th>",
                self.node_names.get(&neighbor.index).unwrap()
            )?;

            for v in &neighbor.dv {
                writeln!(writer, "\t\t<td>{}</td>", v.write_html_long(&self.node_names))?;
            }

            writeln!(writer, "\t</tr>")?;
        }

        writeln!(writer, "</table>")?;

        Ok(())
    }

    fn build_world(
        &self,
        relations: &HashMap<(usize, usize), W>,
        main_dvs: &HashMap<usize, Vec<DVValue<W>>>,
        inbox_dvs: &HashMap<usize, Vec<DVValue<W>>>,
        advance_generation: bool
    ) -> Self {
        let mut new_state: HashMap<String, Node<W>> = HashMap::new();

        for (name, node) in &self.nodes {
            let mut neighbors:Vec<Neighbor<W>> = Vec::new();

            for ((node_a, node_b), new_w) in relations {
                if *node_a == node.index {
                    neighbors.push(Neighbor{
                        index: *node_b,
                        direct_cost: new_w.to_owned(),
                        dv: inbox_dvs.get(node_b).unwrap().to_owned()
                    });
                }
            }

            neighbors.sort_by_key(|n| n.index);

            new_state.insert(name.to_string(), Node {
                name: node.name.to_owned(),
                dv: main_dvs.get(&node.index).unwrap().to_owned(),
                index: node.index,
                neighbors
            });
        }

        let generation =
            if advance_generation {
                self.generation + 1
            } else {
                self.generation
            };

        World { nodes: new_state, generation, node_names: self.node_names.to_owned() }
    }

    fn copy_relations(&self) -> HashMap<(usize, usize), W> {
        let mut relations: HashMap<(usize, usize), W> = HashMap::new();

        for (_, node) in &self.nodes {
            for neighbors in &node.neighbors {
                relations.insert((node.index, neighbors.index), neighbors.direct_cost.to_owned());
            }
        }

        relations
    }

    fn copy_dvs(&self) -> HashMap<usize, Vec<DVValue<W>>> {
        let mut dvs: HashMap<usize, Vec<DVValue<W>>> = HashMap::new();

        for (_, node) in &self.nodes {
            let mut dv = Vec::new();

            for v in &node.dv {
                dv.push(v.clone());
            }

            dvs.insert(node.index, dv);
        }

        dvs
    }

    fn apply_operations<Writer:Write>(&self, writer:&mut Writer, operations: Vec<Operation<W>>) -> Result<Self, Box<dyn Error>> {
        let mut relations: HashMap<(usize, usize), W> = self.copy_relations();
        let mut new_dvs: HashMap<usize, Vec<DVValue<W>>> = self.copy_dvs();

        for op in operations {
            match op {
                Operation::ChangeWeight(node_a, node_b, new_w) => {
                    relations.insert((node_a, node_b), new_w.to_owned());
                    relations.insert((node_b, node_a), new_w.to_owned());

                    new_dvs.insert(
                        node_a,
                        modify_dv(
                            new_dvs.get(&node_a).unwrap(),
                            node_b,
                            new_w.to_owned(),
                        ),
                    );

                    new_dvs.insert(
                        node_b,
                        modify_dv(
                            new_dvs.get(&node_b).unwrap(),
                            node_a,
                            new_w.to_owned(),
                        ),
                    );
                }
            }
        }

        self.build_world(
            &relations,
            &new_dvs,
            &self.copy_dvs(),
            false
        ).print_state(writer)?;

        Ok(self.build_world(
            &relations,
            &new_dvs,
            &new_dvs,
            false
        ))
    }

    fn sorted_nodes(&self) -> Vec<&Node<W>> {
        let mut indexes:Vec<&usize> = self.node_names.keys().collect();
        indexes.sort();

        let mut nodes:Vec<&Node<W>> = Vec::new();

        for index in indexes {
            let node_name = self.node_names.get(index).unwrap();

            nodes.push(self.nodes.get(node_name).unwrap());
        }

        nodes
    }

    fn print_state<Writer: Write>(&self, writer: &mut Writer) -> Result<(), Box<dyn Error>> {
        writeln!(writer, "<h2>t={}</h2>", self.generation)?;

        for node in self.sorted_nodes() {
            self.print_node(writer, node, None)?;
        }

        Ok(())
    }

    fn run_simulation<Writer: Write>(&self, writer: &mut Writer) -> Result<NewState<W>, Box<dyn Error>> {
        writeln!(writer, "<h2>t={}</h2>", self.generation + 1)?;

        let mut changed_nodes: HashSet<usize> = HashSet::new();
        let mut new_dvs: HashMap<usize, Vec<DVValue<W>>> = self.copy_dvs();

        for node in self.sorted_nodes() {
            let mut new_dv = Vec::new();
            let mut index: usize = 0;

            let mut lines: Vec<String> = Vec::new();

            for v_old in &node.dv {
                if index == node.index {
                    new_dv.push(DVValue::SameNode);
                } else {
                    // For debug printing
                    let mut formula =
                        HtmlFormula::new(index, node.index);

                    for neighbour in &node.neighbors {
                        if neighbour.index == index {
                            formula.add_direct(
                                neighbour.index,
                                node.index,
                                neighbour
                                    .direct_cost
                                    .to_owned()
                            );
                        } else {
                            formula.add_indirect(
                                neighbour.index,
                                node.index,
                                neighbour
                                    .direct_cost
                                    .to_owned() ,
                                index,
                                neighbour.index,
                                neighbour.dv
                                    .get(index)
                                    .unwrap()
                                    .into()
                            );
                        }
                    }

                    let v = formula.min_vector();

                    if v != v_old.to_owned() {
                        changed_nodes.insert(node.index);
                    }

                    new_dv.push(v);
                    lines.push(formula.render(&self.node_names));
                }

                index += 1;
            }

            self.print_node(writer, node, Some(&new_dv))?;
            writeln!(writer, "<div class=\"details\">")?;
            for line in lines {
                writeln!(writer, "\t<div>{}</div>", line)?;
            }
            writeln!(writer, "</div>")?;

            new_dvs.insert(node.index, new_dv);
        }

        if changed_nodes.is_empty() {
            Ok(NewState::NotChanged)
        } else {
            Ok(NewState::Changed(self.build_world(
                &self.copy_relations(),
                &new_dvs,
                &new_dvs,
                true
            )))
        }
    }
}



fn run_until_stable<Writer: Write>(writer: &mut Writer, world: World<u32>) -> Result<World<u32>, Box<dyn Error>> {
    match world.run_simulation(writer)? {
        NewState::Changed(w2) => run_until_stable(writer, w2),
        // When no-change advance the generation on by 1
        NewState::NotChanged => Ok(World{
            node_names: world.node_names,
            nodes: world.nodes,
            generation: world.generation+1
        })
    }
}

fn exc2(p:&Path) -> Result<(), Box<dyn Error>> {
    let mut html_file = HtmlFile::new(p.join("exc2.html"))?;

    let world: World<u32> = World::new(vec!("A", "B", "C", "D", "E", "F", "G", "H"));
    //println!("op:{:#?}", world.add_interface("A","B",12)?);

    let init = world.apply_operations(&mut html_file, vec!(
        world.add_interface("A", "D", 3)?,
        world.add_interface("A", "G", 1)?,
        world.add_interface("B", "E", 2)?,
        world.add_interface("B", "H", 1)?,
        world.add_interface("C", "D", 1)?,
        world.add_interface("C", "F", 2)?,
        world.add_interface("D", "G", 6)?,
        world.add_interface("D", "F", 5)?,
        world.add_interface("E", "F", 1)?,
        world.add_interface("E", "H", 3)?,
        world.add_interface("F", "H", 8)?,
    ))?;

    let stable = run_until_stable(&mut html_file, init)?;

    let new_init = stable.apply_operations(&mut html_file, vec!(
        world.add_interface("C", "F", 30)?
    ))?;

    run_until_stable(&mut html_file, new_init)?;

    Ok(())
}

fn exc3(p:&Path) -> Result<(), Box<dyn Error>> {
    let mut html_file = HtmlFile::new(p.join("exc3.html"))?;

    let world: World<u32> = World::new(vec!("A", "B", "C", "D"));
    //println!("op:{:#?}", world.add_interface("A","B",12)?);

    let init = world.apply_operations(&mut html_file, vec!(
        world.add_interface("A", "B", 2)?,
        world.add_interface("B", "C", 7)?,
        world.add_interface("C", "D", 4)?,
        world.add_interface("A", "D", 8)?,
        world.add_interface("B", "D", 9)?
    ))?;

    let world1_done = run_until_stable(&mut html_file, init)?;

    let world2 = world1_done.apply_operations(&mut html_file, vec!(
        world1_done.add_interface("B", "D", 80)?
    ))?;

    run_until_stable(&mut html_file, world2)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let output_path = Path::new("/Users/ilya/Desktop/dvr");
    exc2(output_path)?;
    exc3(output_path)?;

    Ok(())
}
