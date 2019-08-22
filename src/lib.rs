//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright 2019 Joyent, Inc.
//
extern crate serde_derive;
extern crate serde;
extern crate serde_xml_rs;

extern crate topo_digraph_xml;
use topo_digraph_xml::{
    NvlistXmlArrayElement,
    TopoDigraphXML,
    PG_NAME,
    PG_VALS
};

//extern crate nvpair;

extern crate svg;
use svg::Document;
use svg::node::element::{
    Group,
    Line,
    Rectangle,
    Script,
    Text
};

use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use std::fmt;
use std::fs;

//
// Constants for topo node names in SAS scheme topology
//
pub const INITIATOR: &'static str = "initiator";
pub const PORT: &'static str = "port";
pub const EXPANDER: &'static str = "expander";
pub const TARGET: &'static str = "target";

const ONCLICK: &'static str = r#"<![CDATA[
function showInfo(evt) {
    var group = evt.target.parentElement;
    var fmri = group.getAttribute("fmri");
    console.log('fmri is %s', fmri);
}
]]>
"#;

const ONMOUSEOVER: &'static str = r#"<![CDATA[
function highlightVertex(evt) {
    var rect = evt.target;
    rect.setAttribute("fill", "cyan");
    console.log('hello there');
}
]]>
"#;

const ONMOUSEOUT: &'static str = r#"<![CDATA[
function unHighlightVertex(evt) {
    var rect = evt.target;
    rect.setAttribute("fill", "none");
    console.log('goodbye');
}
]]>
"#;

#[derive(Debug)]
struct SimpleError(String);

impl Error for SimpleError {}

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
struct SasGeometry {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl SasGeometry {
    fn new(x: u32, y: u32, width: u32, height: u32)
        -> SasGeometry {

        SasGeometry {
            x, y, width, height,
        }
    }
}

#[derive(Debug)]
struct SasDigraphProperty {
    name: String,
    value: String,
}

impl SasDigraphProperty {
    fn new(name: String, value: String) -> SasDigraphProperty {
        SasDigraphProperty { name, value }
    }
}

#[derive(Debug)]
struct SasDigraphVertex {
    fmri: String,
    name: String,
    instance: u64,
    properties: Vec<SasDigraphProperty>,
    geometry: SasGeometry,
    outgoing_edges: Option<Vec<String>>,
}

impl SasDigraphVertex {
    fn new(fmri: String, name: String, instance: u64, outgoing_edges: Option<Vec<String>>)
        -> SasDigraphVertex {

        let properties = Vec::new();
        let geometry = SasGeometry::new(0, 0, 0, 0);
        SasDigraphVertex {
            fmri, name, instance, properties, geometry, outgoing_edges
        }
    }
}

#[derive(Debug)]
struct SasDigraph {
    // hashmap of vertices, hashed by FMRI
    vertices: HashMap<String, SasDigraphVertex>,
    // array of initiator FMRIs
    initiators: Vec<String>,
}

impl SasDigraph {
    fn new() -> SasDigraph {
        let vertices = HashMap::new();
        let initiators = Vec::new();

        SasDigraph {
            vertices,
            initiators,
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub svg_path: String,
    pub xml_path: String,
}

impl Config {
    pub fn new(svg_path: String, xml_path: String) -> Config {
        Config { svg_path, xml_path }
    }
}

fn visit_vertex(vertices: &HashMap<String, SasDigraphVertex>, 
    vtx: &SasDigraphVertex, column_hash: &mut HashMap<u32, Vec<String>>,
    depth: &u32) -> Result<u32, Box<dyn Error>> {
    
    let mut max_depth = depth + 1;

    column_hash.entry(max_depth)
        .or_insert_with(Vec::new)
        .push(vtx.fmri.clone());

    if vtx.outgoing_edges.is_some() {
        for edge in vtx.outgoing_edges.as_ref().unwrap() {
            let next_vtx = match vertices.get(&edge.to_string()) {
                Some(entry) => {
                    entry.clone()
                }
                None => {
                    return Err(Box::new(SimpleError("failed to lookup vertex".to_string())));
                }
            };
            let rc = visit_vertex(vertices, next_vtx, column_hash, &(depth + 1))?;
            if rc > max_depth {
                max_depth = rc;
            }
        }
    }
    Ok(max_depth)
}

fn build_info_panel() -> Result<Group, Box<dyn Error>> {

    let info_x = 10;
    let info_y = 10;
    let info_rect = Rectangle::new()
        .set("x", info_x)
        .set("y", info_y)
        .set("width", 500)
        .set("height", 1000)
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 3);

    let txt = svg::node::Text::new("Node Properties:");
    let info_label = Text::new()
        .set("x", info_x + 5)
        .set("y", info_y + 20)
        .add(txt);

    let info_group = Group::new()
        .add(info_rect)
        .add(info_label);

    Ok(info_group)
}

//
// Generates an SVG representation of the directed graph and save it to a file.
//
fn build_svg(config: &Config, digraph: &mut SasDigraph) -> Result<(), Box<dyn Error>> {
    
    let mut max_depth: u32 = 0;
    let mut max_height: usize = 0;
    let mut column_hash: HashMap<u32, Vec<String>> = HashMap::new();
    let depth: u32 = 0;

    //
    // First we iterate over all of the paths through the digraph starting from
    // the initiator vertices.  There are two purposes here:
    //
    // The first is to calculate the maximum depth (width) of the graph.
    // The second is to create a hash map of vertex FMRIs, hashed by their
    // depth.
    //
    // We'll iterate through that hash to determine the maximum height of the
    // graph, and then again when we construct the SVG elements.
    //
    // Based on the maximum depth and height, we'll divide the document into a
    // grid and use that to determine the size and placement of the various SVG
    // elements.
    //
    for fmri in &digraph.initiators {
        println!("initiator: {}", fmri);
        let vtx = match digraph.vertices.get(&fmri.to_string()) {
            Some(entry) => {
                entry.clone()
            }
            None => {
                return Err(Box::new(SimpleError("failed to lookup vertex".to_string())));
            }
        };
    
        let rc = visit_vertex(&digraph.vertices, vtx, &mut column_hash, &depth)?;
        if rc > max_depth {
            max_depth = rc;
        }
    }

    for i in 1..(max_depth + 1) {
        let height = match column_hash.get(&i) {
            Some(entry) => {
                entry.len()
            }
            None => { 0 }
        };
        println!("depth: {} has height {}", i, height);
        if height > max_height {
            max_height = height;
        }
    }
    println!("max_depth: {}", max_depth);
    println!("max_height: {}", max_height);

    let on_click = Script::new(ONCLICK)
        .set("type", "application/ecmascript");

    let on_mouseover = Script::new(ONMOUSEOVER)
        .set("type", "application/ecmascript");

    let on_mouseout = Script::new(ONMOUSEOUT)
        .set("type", "application/ecmascript");

    let info_group = build_info_panel()?;

    let mut document = Document::new()
        .set("overflow", "scroll")
        .set("viewbox", (0, 0, (100 * max_depth), (250 * max_height)))
        .add(on_click)
        .add(on_mouseover)
        .add(on_mouseout)
        .add(info_group);

    let vtx_width = 180;
    let vtx_height = 70;

    //
    // Generate the SVG elements for all the vertices.
    //
    for depth in 1..=max_depth {
        let vertices = column_hash.get(&depth).unwrap();
        for index in 0..vertices.len() {
            let height: u32 = (index + 1).try_into().unwrap();
            let vtx_fmri: String = vertices[index].to_string();
            let vtx = digraph.vertices.get_mut(&vtx_fmri).unwrap();
            
            let x_margin = 600;
            let y_margin = 10;
            let x = ((depth - 1) * 250) + x_margin;
            
            let y_factor: u32 = match height {
                1 => { 1 }
                _ => { (max_height / vertices.len()).try_into().unwrap() }
            };
            let y = ((height - 1) * 100 * y_factor) + y_margin;

            println!("VERTEX: fmri: {}, depth: {}, height: {}, x: {}, y: {}", vtx_fmri,
                depth, height, x, y);
            let rect = Rectangle::new()
                .set("x", x)
                .set("y", y)
                .set("width", vtx_width)
                .set("height", vtx_height)
                .set("fill", "none")
                .set("stroke", "black")
                .set("stroke-width", 3)
                .set("onmouseover", "highlightVertex(evt)")
                .set("onmouseout", "unHighlightVertex(evt)");

            vtx.geometry.x = x;
            vtx.geometry.y = y.try_into().unwrap();
            vtx.geometry.width = vtx_width;
            vtx.geometry.height = vtx_height;

            let txt = svg::node::Text::new(format!("{}", vtx.name));
            let name_label = Text::new()
                .set("x", x + 10)
                .set("y", y + 20)
                .add(txt);

            let txt = svg::node::Text::new(format!("{:x}", vtx.instance));
            let inst_label = Text::new()
                .set("x", x + 10)
                .set("y", y + 50)
                .add(txt);

            let vtx_group = Group::new()
                .set("onclick", "showInfo(evt)")
                .set("fmri", vtx_fmri)
                .add(rect)
                .add(name_label)
                .add(inst_label);
            
            document = document.add(vtx_group);
        }
    }

    //
    // Generate the SVG elements for all of the edges
    //
    for depth in 1..=max_depth {
        let vertices = column_hash.get(&depth).unwrap();
        for index in 0..vertices.len() {
            let vtx_fmri: String = vertices[index].to_string();
            let vtx = digraph.vertices.get(&vtx_fmri).unwrap();

            if vtx.outgoing_edges.is_none() {
                println!("no edges: {}", vtx_fmri);
                break;
            }
            let start_x1 = vtx.geometry.x + vtx_width;
            let start_y1: u32 = vtx.geometry.y + (vtx_height / 2);
            let start_x2 = start_x1 + 50;
            let start_y2 = start_y1;
            let line = Line::new()
                .set("x1", start_x1)
                .set("y1", start_y1)
                .set("x2", start_x2)
                .set("y2", start_y2)
                .set("stroke", "black")
                .set("stroke-width", "2");

            document = document.add(line);
            println!("\nEDGE: from: {}", vtx_fmri);
            
            for edge_fmri in vtx.outgoing_edges.as_ref().unwrap() {
                println!("        to: {}", edge_fmri);
                let edge_vtx = digraph.vertices.get(edge_fmri).unwrap();
                let mid_x1 = start_x2;
                let mid_y1 = start_y2;
                let mid_x2 = start_x2;
                let mid_y2 = edge_vtx.geometry.y + (vtx_height / 2);

                let line = Line::new()
                    .set("x1", mid_x1)
                    .set("y1", mid_y1)
                    .set("x2", mid_x2)
                    .set("y2", mid_y2)
                    .set("stroke", "black")
                    .set("stroke-width", "2");

                document = document.add(line);

                let end_x1 = start_x2;
                let end_y1 = edge_vtx.geometry.y + (vtx_height / 2);
                let end_x2 = edge_vtx.geometry.x;
                let end_y2 = end_y1;

                let line = Line::new()
                    .set("x1", end_x1)
                    .set("y1", end_y1)
                    .set("x2", end_x2)
                    .set("y2", end_y2)
                    .set("stroke", "black")
                    .set("stroke-width", "2");

                document = document.add(line);
            }
        }
    }

    svg::save(&config.svg_path, &document)?;

    Ok(())
}

pub fn run(config: &Config) -> Result<(), Box<dyn Error>> {
    
    //
    // Read in the serialized (XML) representation of a SAS topology and
    // deserialize it into a TopoDigraphXML structure.
    //
    let xml_contents = fs::read_to_string(&config.xml_path)?;
    let sasxml: TopoDigraphXML = serde_xml_rs::from_str(&xml_contents)?;

    let mut digraph = SasDigraph::new();

    //
    // Iterate through the TopoDigraphXML and recreate the SAS topology in the
    // form for a SasDigraph structure.
    //
    for vtxxml in sasxml.vertices.vertex {

        let instance = u64::from_str_radix(&vtxxml.instance, 16)?;

        let mut vtx = match vtxxml.outgoing_edges {
            Some(outgoing_edges) => {
                let mut edges = Vec::new();
                for edgexml in outgoing_edges.edges {
                    edges.push(edgexml.fmri);
                }
                SasDigraphVertex::new(vtxxml.fmri, vtxxml.name, instance,
                    Some(edges))
            }
            None => {
                SasDigraphVertex::new(vtxxml.fmri, vtxxml.name, instance,
                    None)
            }
        };

        for pgnvl in vtxxml.propgroups {
            let pgarr = pgnvl.nvlist_elements.unwrap();
            //for pg in pgarr {
            //}
        }

        if vtx.name == INITIATOR {
            digraph.initiators.push(vtx.fmri.clone());
        }
        digraph.vertices.insert(vtx.fmri.clone(), vtx);
    }

    //
    // Generate an SVG from the SasDigraph structure and save it to the
    // specified file.
    //
    build_svg(config, &mut digraph)?;
    
    Ok(())
}
