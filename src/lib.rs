//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright 2019 Joyent, Inc.
//
extern crate log;
extern crate env_logger;

use log::debug;

extern crate serde_derive;
extern crate serde;
extern crate serde_xml_rs;

extern crate topo_digraph_xml;
use topo_digraph_xml::{
    NvlistXmlArrayElement,
    TopoDigraphXML,
    PG_NAME,
    PG_VALS,
    PROP_NAME,
    PROP_VALUE,
};

extern crate svg;
use svg::Document;
use svg::node::element::{
    ForeignObject,
    Group,
    Line,
    Rectangle,
    Script,
    Text
};

use std::cmp;
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;

//
// Constants for topo node names in SAS scheme topology
//
pub const INITIATOR: &str = "initiator";
pub const PORT: &str = "port";
pub const EXPANDER: &str = "expander";
pub const TARGET: &str = "target";

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
    // machine nodename
    nodename: String,
    // OS version
    os_version: String,
    // time of snapshot in ISO-8601 format
    timestamp: String,
    // hashmap of vertices, hashed by FMRI
    vertices: HashMap<String, SasDigraphVertex>,
    // array of initiator FMRIs
    initiators: Vec<String>,
}

impl SasDigraph {
    fn new(nodename: String, os_version: String, timestamp: String) -> SasDigraph {
        let vertices = HashMap::new();
        let initiators = Vec::new();

        SasDigraph {
            nodename,
            os_version,
            timestamp,
            vertices,
            initiators,
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub html_path: String,
    pub xml_path: String,
}

impl Config {
    pub fn new(html_path: String, xml_path: String) -> Config {
        Config { html_path, xml_path }
    }
}

//
// Parse an NvlistXmlArrayElement representing a topo property, extract the
// prop name and value (as a string) and return a SasDigraphProperty.
//
fn parse_prop(nvl: &NvlistXmlArrayElement) -> Result<SasDigraphProperty, Box<dyn Error>> {
    let mut propname: Option<String> = None;
    let mut propval: Option<String> = None;

    for nvpair in &nvl.nvpairs {

        match nvpair.name.as_ref().unwrap().as_ref() {
            PROP_NAME => { propname = Some(nvpair.value.as_ref().unwrap().clone()); }
            PROP_VALUE => {
                if nvpair.nvpair_elements.is_some() {
                    //
                    // If nvpair_elements is something then this is an array
                    // type in which case we iterate through the child nvpairs
                    // and create a string with all the array values,
                    // delimited by a comma.
                    //
                    let mut valarr = Vec::new();
                    for elem in nvpair.nvpair_elements.as_ref().unwrap() {
                        valarr.push(elem.value.as_ref().unwrap().clone());
                    }
                    propval = Some(valarr.join(","));
                } else {
                    propval = Some(nvpair.value.as_ref().unwrap().clone());
                }
            }
            _ => {}
        }
    }

    if propname.is_none() || propval.is_none() {
        Err(Box::new(SimpleError(format!("malformed property value nvlist: {:?}", nvl))))
    } else {
        Ok(SasDigraphProperty::new(propname.unwrap(), propval.unwrap()))
    }
}

fn visit_vertex(vertices: &HashMap<String, SasDigraphVertex>, 
    vtx: &SasDigraphVertex, column_hash: &mut HashMap<u32, Vec<String>>,
    depth: u32) -> Result<u32, Box<dyn Error>> {
    
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
            let rc = visit_vertex(vertices, next_vtx, column_hash, depth + 1)?;
            if rc > max_depth {
                max_depth = rc;
            }
        }
    }
    Ok(max_depth)
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
    // First we create a hidden element that we can attach the host information
    // prooperties to.  The JS code will reference those to populate the Host
    // Information table,
    //
    let hostinfo = Rectangle::new()
        .set("x", 1)
        .set("y", 1)
        .set("width", 1)
        .set("height", 1)
        .set("visibility", "hidden")
        .set("id", "hostprops")
        .set("nodename", digraph.nodename.clone())
        .set("os-version", digraph.os_version.clone())
        .set("timestamp", digraph.timestamp.clone());
    
    //
    // Next we iterate over all of the paths through the digraph starting from
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
        debug!("initiator: {}", fmri);
        let vtx = match digraph.vertices.get(&fmri.to_string()) {
            Some(entry) => {
                entry.clone()
            }
            None => {
                return Err(Box::new(SimpleError("failed to lookup vertex".to_string())));
            }
        };
    
        let rc = visit_vertex(&digraph.vertices, vtx, &mut column_hash, depth)?;
        if rc > max_depth {
            max_depth = rc;
        }
    }

    for i in 1..=max_depth {
        let height = match column_hash.get(&i) {
            Some(entry) => {
                entry.len()
            }
            None => { 0 }
        };
        debug!("depth: {} has height {}", i, height);
        if height > max_height {
            max_height = height;
        }
    }
    debug!("max_depth: {}", max_depth);
    debug!("max_height: {}", max_height);

    let mut script = String::new();
    script.push_str("<![CDATA[");
    let js_code = include_str!("sastopo2svg.js");
    script.push_str(js_code);
    script.push_str("]]>");

    let on_click = Script::new(script)
        .set("type", "application/ecmascript");

    let html_code = include_str!("sastopo2svg.html");
    let html_txt = svg::node::Text::new(html_code);
    let foreign = ForeignObject::new()
        .set("x", 10)
        .set("y", 10)
        .set("height", 700)
        .set("width", 900)
        .add(html_txt);

    let mut document = Document::new()
        .set("overflow", "scroll")
        .set("viewbox", (0, 0, (100 * max_depth), (250 * max_height)))
        .add(on_click)
        .add(foreign)
        .add(hostinfo);

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
            
            let x_margin = 850;
            let y_margin = 10;
            let x = ((depth - 1) * 250) + x_margin;
            
            let y_factor: u32 = match height {
                1 => { 1 }
                _ => { (max_height / vertices.len()).try_into().unwrap() }
            };
            let y = ((height - 1) * 100 * y_factor) + y_margin;

            debug!("VERTEX: fmri: {}, depth: {}, height: {}, x: {}, y: {}", vtx_fmri,
                depth, height, x, y);
            let rect = Rectangle::new()
                .set("x", x)
                .set("y", y)
                .set("width", vtx_width)
                .set("height", vtx_height)
                .set("fill", "white")
                .set("stroke", "black")
                .set("stroke-width", 3);

            vtx.geometry.x = x;
            vtx.geometry.y = y.try_into().unwrap();
            vtx.geometry.width = vtx_width;
            vtx.geometry.height = vtx_height;

            let txt = svg::node::Text::new(vtx.name.to_string());
            let name_label = Text::new()
                .set("x", x + 10)
                .set("y", y + 20)
                .set("font-family", "Courier New, Courier, monospace")
                .add(txt);

            let txt = svg::node::Text::new(format!("{:x}", vtx.instance));
            let inst_label = Text::new()
                .set("x", x + 10)
                .set("y", y + 50)
                .set("font-family", "Courier New, Courier, monospace")
                .add(txt);

            let mut vtx_group = Group::new()
                .set("onclick", "showInfo(evt)")
                .set("name", vtx.name.clone())
                .set("fmri", vtx_fmri)
                .add(rect)
                .add(name_label)
                .add(inst_label);
            
            for prop in &vtx.properties {
                vtx_group = vtx_group.set(prop.name.clone(), prop.value.clone());
            }

            document = document.add(vtx_group);
        }
    }

    //
    // Generate the SVG elements for all of the edges
    //
    for depth in 1..=max_depth {
        let vertices = column_hash.get(&depth).unwrap();
        for v in vertices {
	    let vtx_fmri: String = v.to_string();
            let vtx = digraph.vertices.get(&vtx_fmri).unwrap();

            if vtx.outgoing_edges.is_none() {
                continue;
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

            for edge_fmri in vtx.outgoing_edges.as_ref().unwrap() {
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

    let mut svg_path = config.html_path.clone();
    svg_path.push_str(".svg");
    svg::save(&svg_path, &document)?;

    //
    // The SVG can be quite large depending on the size of the SAS fabric.
    // So to allow it to be more easily viewable in a browser, we embed the
    // SVG in a scrollable HTML iframe.
    //
    let svg_width = cmp::max(2000, max_depth * 350);
    let svg_height = cmp::max(1100, max_height * 105);

    let mut htmlfile = fs::File::create(&config.html_path)?;
    htmlfile.write_fmt(format_args!(
        "<html><title>SAS Topology</title><body bgcolor=\"EEEEEE\">\n"))?;
    htmlfile.write_fmt(format_args!(
        "<iframe src=\"{}\" width={} height={} scrollable=\"yes\" frameborder=\"no\" />",
        svg_path, svg_width, svg_height))?;
    htmlfile.write_fmt(format_args!("</body></html>\n"))?;

    Ok(())
}

pub fn run(config: &Config) -> Result<(), Box<dyn Error>> {
    
    //
    // Read in the serialized (XML) representation of a SAS topology and
    // deserialize it into a TopoDigraphXML structure.
    //
    let xml_contents = fs::read_to_string(&config.xml_path)?;
    let sasxml: TopoDigraphXML = serde_xml_rs::from_str(&xml_contents)?;

    let mut digraph = SasDigraph::new(sasxml.nodename, sasxml.os_version, sasxml.timestamp);

    //
    // Iterate through the TopoDigraphXML and recreate the SAS topology in the
    // form of a SasDigraph structure.
    //
    for vtxxml in sasxml.vertices.vertex {

	    // Convert hex string to a u64, skipping the leading '0x'
        let instance = u64::from_str_radix(&vtxxml.instance[2..], 16)?;

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

        //
        // The XML contains a set of nested NvpairXML structures representing
        // the node property groups and their contains properties.  We descend
        // through these to build an array of SasDigraphProperty structs which
        // will contains a subset of properties that we want to display when
        // the vertex is clicked on.
        //
        for pgnvl in vtxxml.propgroups {
            let pgarr = pgnvl.nvlist_elements.unwrap();
            for pg in pgarr {
                let mut owned1;
                let mut owned2;

                let mut props : Option<&Vec<NvlistXmlArrayElement>> = None;
                let mut pgname : &str = "";
                for pgnvp in pg.nvpairs {
                    match pgnvp.name.unwrap().as_ref() {
                        PG_NAME => {
                            owned1 = pgnvp.value.unwrap();
                            pgname = owned1.as_ref();
                        }
                        PG_VALS => {
                            if pgnvp.nvlist_elements.is_some() {
                                owned2 = pgnvp.nvlist_elements.unwrap();
                                props = Some(owned2.as_ref());
                            }
                        }
                        _ => {
                            return Err(Box::new(
                                SimpleError("Unexpected nvpair name".to_string())))
                            }
                    }
                }

                // Sanity check against malformed XML
                if pgname == "" {
                    return Err(Box::new(SimpleError(
                        format!("malformed propgroup, {} not set", PG_NAME))));
                } else if props.is_none() {
                    /*return Err(Box::new(SimpleError(
                        format!("malformed propgroup, {} not set", PG_VALS))));*/
                    continue;
                }

                //
                // The only things in the protocol property group is an nvlist
                // representation of the FMRI, which we don't need as we
                // already have the FMRI as a string in a separate field.
                //
                if pgname == "protocol" {
                    continue;
                }

                for propnvl in props.unwrap() {
                    let prop = parse_prop(&propnvl)?;
                    vtx.properties.push(prop);
                }
            }
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
