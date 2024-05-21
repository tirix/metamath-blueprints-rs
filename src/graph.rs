use crate::item::{ItemState, ItemType};
use crate::project::Project;
use crate::{Error, Result};
/// Dependency Graph Rendering
use layout::backends::svg::SVGWriter;
use layout::core::base::Orientation;
use layout::core::color::Color;
use layout::core::geometry::{get_size_for_str, pad_shape_scalar};
use layout::core::style::*;
use layout::std_shapes::shapes::*;
use layout::topo::layout::VisualGraph;
use regex::Regex;
use std::collections::HashMap;

impl Project {
    pub(crate) fn render_dependency_graph(&self) -> Result<String> {
        // Create a new graph:
        let mut vg = VisualGraph::new(Orientation::TopToBottom);
        let font_size = 15;
        let mut nodes_by_name = HashMap::new();

        for item in &self.items {
            let size = pad_shape_scalar(get_size_for_str(&item.name, font_size), 30.);
            let look = StyleAttr::new(
                match item.info.state {
                    ItemState::ReadyForStmt => Color::fast("blue"),
                    ItemState::StmtFormalized => Color::fast("green"),
                    ItemState::Draft | ItemState::ReadyForProof | ItemState::Formalized => {
                        Color::fast("black")
                    }
                },
                1,
                Option::Some(match item.info.state {
                    ItemState::Draft | ItemState::ReadyForStmt | ItemState::StmtFormalized => {
                        Color::fast("white")
                    }
                    ItemState::ReadyForProof => Color::new(0xA3D6FFFF),
                    ItemState::Formalized => Color::new(0x9CEC8BFF),
                }),
                0,
                font_size,
            );
            let shape = match item.info.r#type {
                ItemType::Theorem => ShapeKind::new_box(&item.name),
                ItemType::Definition => ShapeKind::new_circle(&item.name),
            };
            let element = Element::create(
                shape,
                look,
                Some(format!("{}.html", &item.name)),
                Orientation::LeftToRight,
                size,
            );
            let handle = vg.add_node(element);
            nodes_by_name.insert(&item.name, handle);
        }

        for item in &self.items {
            for dependency in &item.info.dependencies {
                let handle0 = *nodes_by_name.get(&item.name).ok_or(Error::Custom(format!(
                    "Could not find node for item {}",
                    item.name
                )))?;
                let handle1 = *nodes_by_name.entry(dependency).or_insert_with(|| {
                    let size = pad_shape_scalar(get_size_for_str(dependency, font_size), 30.);
                    let look = StyleAttr::new(
                        Color::fast("black"),
                        1,
                        Option::Some(Color::new(0x9CEC8BFF)),
                        0,
                        font_size,
                    );
                    let shape = ShapeKind::new_box(dependency);
                    let element = Element::create(
                        shape,
                        look,
                        Some(format!("{}.html", &dependency)),
                        Orientation::LeftToRight,
                        size,
                    );
                    vg.add_node(element)
                });
                let arrow = Arrow::new(
                    LineEndKind::None,
                    LineEndKind::Arrow,
                    LineStyleKind::Normal,
                    "",
                    &StyleAttr::new(
                        Color::fast("black"),
                        1,
                        Option::Some(Color::fast("white")),
                        0,
                        font_size,
                    ),
                    &None,
                    &None,
                );
                vg.add_edge(arrow, handle0, handle1);
            }
        }

        // Render the graph in SVG.
        let mut svg = SVGWriter::new();
        vg.do_it(false, false, false, &mut svg);

        // Return the output.
        let svg_output = svg.finalize();
        Ok(Regex::new(r#"svg width="[0-9\.]+" height="[0-9\.]+""#)
            .unwrap()
            .replace(&svg_output, "svg")
            .to_string())
    }
}
