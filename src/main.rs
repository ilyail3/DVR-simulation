use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Display;
use std::io::Write;
use std::ops::Add;
use std::fs::File;

#[derive(Debug, Clone, PartialEq)]
enum DVValue<W: PartialOrd + Clone + Add<Output=W> + Display> {
    Infinity,
    Distance(W, usize),
    SameNode,
}


impl<W: PartialOrd + Clone + Add<Output=W> + Display> DVValue<W> {
    fn write_html(&self) -> String {
        match self {
            DVValue::Infinity => String::from("&infin;"),
            DVValue::Distance(v, _) => format!("{}", v),
            DVValue::SameNode => String::from("0")
        }
    }

    fn write_html_long(&self, names: &HashMap<usize, String>) -> String {
        match self {
            DVValue::Infinity => String::from("&infin;"),
            DVValue::Distance(v, id) => format!(
                "{}({})",
                v,
                names.get(id).unwrap()
            ),
            DVValue::SameNode => String::from("0")
        }
    }
}

#[derive(Debug, Clone)]
struct Node<W: PartialOrd + Clone + Add<Output=W> + Display> {
    name: String,
    dv: Vec<DVValue<W>>,
    inbox: Vec<(usize, Vec<DVValue<W>>)>,
    index: usize,
    connected: Vec<(usize, W)>,
}

#[derive(Debug)]
enum Operation<W: PartialOrd + Clone + Add<Output=W> + Display> {
    ChangeWeight(usize, usize, W)
}

#[derive(Debug)]
struct World<W: PartialOrd + Clone + Add<Output=W> + Display> {
    nodes: HashMap<String, Node<W>>,
    node_names: HashMap<usize, String>,
    generation: u32,
}

#[derive(Debug)]
enum NewState<W: PartialOrd + Clone + Add<Output=W> + Display> {
    Changed(World<W>),
    NotChanged,
}

fn modify_dv<W: PartialOrd + Clone + Add<Output=W> + Display>(
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


impl<W: PartialOrd + Clone + Add<Output=W> + Display> World<W> {
    pub fn new(node_names: Vec<&str>) -> World<W> {
        let mut result: HashMap<String, Node<W>> = HashMap::with_capacity(node_names.len());
        let mut node_names_out: HashMap<usize, String> = HashMap::new();
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
                inbox: Vec::new(),
                index,
                connected: Vec::new(),
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

    fn print_change<Writer: Write>(&self, writer: &mut Writer, node: &Node<W>, new_dv: &Vec<DVValue<W>>) -> Result<(), Box<dyn Error>> {
        writeln!(writer, "<table>\n\t<tr>")?;
        writeln!(writer, "\t\t<th>{}</th>", node.name)?;
        for (index, _) in new_dv.iter().enumerate() {
            writeln!(writer, "\t\t<th>{}</th>", self.node_names.get(&index).unwrap())?;
        }
        writeln!(writer, "\t</tr>\n\t<tr>\n\t\t<th>{}</th>",node.name)?;
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
        writeln!(writer, "\t</tr>")?;

        for (index, inbox) in &node.inbox {
            writeln!(
                writer,
                "\t<tr>\n\t\t<th>{}</th>",
                self.node_names.get(&index).unwrap()
            )?;

            for (_, inbox_value) in inbox.iter().enumerate() {
                writeln!(writer, "\t\t<td>{}</td>", inbox_value.write_html_long(&self.node_names))?;
            }

            writeln!(writer, "\t</tr>")?;
        }

        writeln!(writer, "</table>")?;

        Ok(())
    }

    fn apply_operations(&self, operations: Vec<Operation<W>>) -> Self {
        let mut changed_nodes: HashSet<usize> = HashSet::new();
        let mut new_state: HashMap<String, Node<W>> = HashMap::new();

        let mut relations: HashMap<(usize, usize), W> = HashMap::new();
        let mut new_dvs: HashMap<usize, Vec<DVValue<W>>> = HashMap::new();

        for (_, node) in &self.nodes {
            for (conn, v) in &node.connected {
                relations.insert((node.index, conn.to_owned()), v.to_owned());
            }

            let mut new_dv = Vec::new();

            for v in &node.dv {
                new_dv.push(v.clone());
            }

            new_dvs.insert(node.index, new_dv);
        }

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

                    changed_nodes.insert(node_a);
                    changed_nodes.insert(node_b);
                }
            }
        }

        for (name, node) in &self.nodes {
            let mut inbox: Vec<(usize, Vec<DVValue<W>>)> = Vec::new();
            let mut connected: Vec<(usize, W)> = Vec::new();

            for ((node_a, node_b), new_w) in &relations {
                if *node_a == node.index {
                    if changed_nodes.contains(node_b) {
                        inbox.push((*node_b, new_dvs.get(node_b).unwrap().to_owned()));
                    }

                    connected.push((*node_b, new_w.to_owned()))
                }
            }

            new_state.insert(name.to_string(), Node {
                name: node.name.to_owned(),
                dv: new_dvs.get(&node.index).unwrap().to_owned(),
                inbox,
                index: node.index,
                connected,
            });
        }

        World { nodes: new_state, generation: self.generation, node_names: self.node_names.to_owned() }
    }

    fn run_simulation<Writer: Write>(&self, writer: &mut Writer) -> Result<NewState<W>, Box<dyn Error>> {
        writeln!(writer, "<h2>t={}</h2>", self.generation)?;

        let mut changed_nodes: HashSet<usize> = HashSet::new();

        let mut relations: HashMap<(usize, usize), W> = HashMap::new();
        let mut new_dvs: HashMap<usize, Vec<DVValue<W>>> = HashMap::new();

        let mut indexes:Vec<&usize> = self.node_names.keys().collect();
        indexes.sort();

        for index in indexes {
            let node = self
                .nodes
                .get(self.node_names.get(index).unwrap())
                .unwrap();

            let mut direct_cost: HashMap<usize, W> = HashMap::new();

            for (conn, v) in &node.connected {
                relations.insert((node.index, conn.to_owned()), v.to_owned());
                direct_cost.insert(conn.to_owned(), v.to_owned());
            }

            let mut new_dv = Vec::new();
            let mut index: usize = 0;

            let mut lines: Vec<String> = Vec::new();

            for v in &node.dv {
                if index == node.index {
                    lines.push(format!("D<sub>{}</sub>({})=0", node.name, node.name));
                    new_dv.push(DVValue::SameNode);
                } else {
                    let mut v = v.clone();

                    // For debug printing
                    let mut line = format!(
                        "d<sub>{}</sub>({})=min(C({},{})",
                        node.name,
                        self.node_names.get(&index).unwrap(),
                        node.name,
                        self.node_names.get(&index).unwrap()
                    );

                    for (node_b, _) in &node.inbox {
                        line += format!(
                            ",C({},{})+d<sub>{}</sub>({})",
                            node.name,
                            self.node_names.get(node_b).unwrap(),
                            self.node_names.get(node_b).unwrap(),
                            self.node_names.get(&index).unwrap()
                        ).as_str();
                    }

                    line += ")=min(";
                    line += v.write_html().as_str();

                    for (node_b, node_b_vector) in &node.inbox {
                        line += ",";
                        line += format!("{}", direct_cost.get(node_b).unwrap()).as_str();
                        line += "+";
                        line += node_b_vector.get(index).unwrap().write_html().as_str();


                        let node_b_cost = match node_b_vector.get(index).unwrap() {
                            DVValue::Infinity => None,
                            DVValue::Distance(old_w, _) =>
                                Some(old_w.to_owned() + direct_cost.get(node_b).unwrap().to_owned()),
                            DVValue::SameNode =>
                                Some(direct_cost.get(node_b).unwrap().to_owned())
                        };

                        if let Some(new_w) = node_b_cost {
                            let replace = match &v {
                                DVValue::Infinity => true,
                                DVValue::Distance(old_w, _) => old_w.to_owned() > new_w,
                                DVValue::SameNode => false
                            };

                            if replace {
                                v = DVValue::Distance(new_w, *node_b);
                                changed_nodes.insert(node.index);
                            }
                        }
                    }

                    line += ")=";
                    line += v.write_html().as_str();

                    new_dv.push(v);
                    lines.push(line);
                }

                index += 1;
            }

            writeln!(writer, "<div class=\"single_table\">")?;
            self.print_change(writer, node, &new_dv)?;
            writeln!(writer, "\t<div class=\"details\">")?;
            for line in lines {
                writeln!(writer, "\t\t<div>{}</div>", line)?;
            }
            writeln!(writer, "\t</div>")?;


            writeln!(writer, "</div>")?;

            new_dvs.insert(node.index, new_dv);
        }

        if changed_nodes.is_empty() {
            Ok(NewState::NotChanged)
        } else {
            let mut new_state: HashMap<String, Node<W>> = HashMap::new();

            for (name, node) in &self.nodes {
                let mut inbox: Vec<(usize, Vec<DVValue<W>>)> = Vec::new();
                let mut connected: Vec<(usize, W)> = Vec::new();

                for ((node_a, node_b), new_w) in &relations {
                    if *node_a == node.index {
                        if changed_nodes.contains(node_b) {
                            inbox.push((*node_b, new_dvs.get(node_b).unwrap().to_owned()));
                        }

                        connected.push((*node_b, new_w.to_owned()))
                    }
                }

                new_state.insert(name.to_string(), Node {
                    name: node.name.to_owned(),
                    dv: new_dvs.get(&node.index).unwrap().to_owned(),
                    inbox,
                    index: node.index,
                    connected,
                });
            }

            Ok(NewState::Changed(World { nodes: new_state, generation: self.generation + 1, node_names: self.node_names.to_owned() }))
        }
    }
}

fn initial_world() -> Result<World<u32>, Box<dyn Error>> {
    let world: World<u32> = World::new(vec!("A", "B", "C", "D", "E", "F", "G", "H"));
    //println!("op:{:#?}", world.add_interface("A","B",12)?);

    Ok(world.apply_operations(vec!(
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
    )))
}

fn run_until_stable<Writer: Write>(writer: &mut Writer, world: World<u32>) -> Result<World<u32>, Box<dyn Error>> {
    match world.run_simulation(writer)? {
        NewState::Changed(w2) => run_until_stable(writer, w2),
        NewState::NotChanged => Ok(world)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let world: World<u32> = initial_world()?;
    let mut file = File::create("/Users/ilya/Desktop/result.html")?;

    writeln!(file, "<!DOCTYPE html>")?;
    writeln!(file, "<html>\n<head>")?;
    writeln!(file, "<link rel=\"stylesheet\" href=\"styles.css\">")?;

    writeln!(file, "</head>\n<body>")?;

    run_until_stable(&mut file, world)?;
    writeln!(file, "</body>\n</html>")?;

    Ok(())
}