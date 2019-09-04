//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright 2019 Joyent, Inc.
//
//
extern crate log;
extern crate env_logger;

extern crate getopts;
use getopts::Options;

use std::env;
use std::process;

extern crate sastopo_xml2svg;

fn usage(progname: &str, opts: &Options) {
    let msg = format!("USAGE: {} -x XML -o SVG", progname);
    print!("{}", opts.usage(&msg));
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let progname = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this usage message");
    opts.optopt("o", "SVG", "pathname of SVG file to create", "SVG");
    opts.optopt("x", "XML", "Output of sastopo -x", "XML");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!(e.to_string()),
    };

    if matches.opt_present("h") {
        usage(&progname, &opts);
        process::exit(2);
    }

    let svg_path = match matches.opt_str("o") {
        Some(path) => path,
        None => {
            eprintln!("-o argument is required");
            usage(&progname, &opts);
            process::exit(2);
        }
    };

    let xml_path = match matches.opt_str("x") {
        Some(path) => path,
        None => {
            eprintln!("-x argument is required");
            usage(&progname, &opts);
            process::exit(2);
        }
    };

    let config = sastopo_xml2svg::Config::new(svg_path, xml_path);

    match sastopo_xml2svg::run(&config) {
        Ok(_r) => {
            process::exit(0);
        }
        Err(e) => {
            eprintln!("An error occurred: {}", e.to_string());
            process::exit(1);
        }
    }
}
