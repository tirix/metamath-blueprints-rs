use crate::{nav::Nav, project::Project};
use clap::ArgMatches;
use colored::Colorize;
use handlebars::Handlebars;
use notify::{RecursiveMode, Watcher};
use pulldown_cmark::Options;
use std::process::exit;
use std::sync::OnceLock;
use std::{
    fs::{self, create_dir_all},
    path::{Path, PathBuf},
};
use thiserror::Error;

mod graph;
mod item;
mod nav;
mod project;

static OUT_DIR: &str = "build";

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error: {0}")]
    Custom(String),
    #[error("TOML Decoding Error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Template Error: {0}")]
    Template(#[from] handlebars::TemplateError),
    #[error("Rendering Error: {0}")]
    Render(#[from] handlebars::RenderError),
    #[error("Watch Error: {0}")]
    Watch(#[from] notify::Error),
}

pub type Result<T = ()> = std::result::Result<T, Error>;

fn main() {
    let cmd = clap::Command::new("metamath-blueprints-rs")
        .bin_name("metamath-blueprints-rs")
        .subcommand_required(true)
        .subcommand(
            clap::command!("build")
                .arg(clap::arg!(<PATH>).value_parser(clap::value_parser!(std::path::PathBuf))),
        )
        .subcommand(
            clap::command!("watch")
                .arg(clap::arg!(<PATH>).value_parser(clap::value_parser!(std::path::PathBuf))),
        );
    let matches = cmd.get_matches();
    let (action, matches): (&dyn Fn(&PathBuf) -> Result, &ArgMatches) = match matches.subcommand() {
        Some(("build", matches)) => (&build, matches),
        Some(("watch", matches)) => (&watch, matches),
        _ => unreachable!("clap should ensure we don't get here"),
    };
    let path = matches.get_one::<std::path::PathBuf>("PATH").unwrap();
    match action(path) {
        Ok(()) => {
            println!("{}", "Complete".green());
            exit(0);
        }
        Err(e) => {
            println!("{}: {}", "failed".red(), e);
            exit(-1);
        }
    }
}

fn watch(path: &PathBuf) -> Result {
    build(path)?;
    println!("Watching blue prints in: {path:?}");
    loop {
        let mut watcher = notify::recommended_watcher(EventHandler {
            path: path.to_path_buf(),
        })?;
        watcher.watch(path, RecursiveMode::Recursive)?;
    }
}

struct EventHandler {
    path: PathBuf,
}

impl notify::EventHandler for EventHandler {
    fn handle_event(&mut self, res: notify::Result<notify::Event>) {
        match res {
            Ok(_event) => {
                //_event.paths.iter().all(|p| p.starts_with(out_path))
                let _ = build(&self.path).inspect_err(|e| println!("{}: {}", "failed".red(), e));
            }
            Err(e) => println!("Watch error: {:?}", e),
        }
    }
}

fn build(path: &PathBuf) -> Result {
    println!("Building blue prints in: {path:?}");
    let projects = traverse(path, Project::from_path)?;
    let nav = Nav {
        projects: &projects,
    };
    let out_path = ensure_dir(path, OUT_DIR)?;
    copy_static_dir(&out_path)?;
    nav.build(path, &out_path)?;
    for project in &projects {
        project.build(&nav, &out_path)?;
    }
    Ok(())
}

fn copy_static_dir(path: &Path) -> Result {
    let static_path = ensure_dir(path, "static")?;
    fs::write(
        static_path.join("favicon.ico"),
        include_bytes!("../static/favicon.ico"),
    )?;
    fs::write(
        static_path.join("Metamath_logo.png"),
        include_bytes!("../static/Metamath_logo.png"),
    )?;
    fs::write(
        static_path.join("metamath.css"),
        include_bytes!("../static/metamath.css"),
    )?;
    fs::write(
        static_path.join("mmlogo.svg"),
        include_bytes!("../static/mmlogo.svg"),
    )?;
    fs::write(
        static_path.join("xits-math.woff"),
        include_bytes!("../static/xits-math.woff"),
    )?;
    Ok(())
}

fn get_cmark_options() -> &'static Options {
    static CMARK_OPTIONS: OnceLock<Options> = OnceLock::new();
    CMARK_OPTIONS.get_or_init({
        || {
            let mut options = Options::empty();
            options.insert(Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS);
            options.insert(Options::ENABLE_STRIKETHROUGH);
            options.insert(Options::ENABLE_MATH);
            options.insert(Options::ENABLE_FOOTNOTES);
            options.insert(Options::ENABLE_GFM);
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_TASKLISTS);
            options
        }
    })
}

fn get_hbs_templates() -> Result<&'static Handlebars<'static>> {
    static HBS_TEMPLATES: OnceLock<Handlebars> = OnceLock::new();
    Ok(HBS_TEMPLATES.get_or_init(|| {
        let mut templates = Handlebars::new();
        templates.register_escape_fn(handlebars::no_escape);
        templates
            .register_template_string("index", include_str!("../templates/index.hbs"))
            .expect("Could not parse index template.");
        templates
            .register_template_string("project", include_str!("../templates/project.hbs"))
            .expect("Could not parse project template.");
        templates
            .register_template_string("item", include_str!("../templates/item.hbs"))
            .expect("Could not parse item template.");
        templates
            .register_partial("nav", include_str!("../templates/nav.hbs"))
            .expect("Could not parse nav partial template.");
        templates
    }))
}

fn ensure_dir(path: &Path, name: &str) -> Result<PathBuf> {
    let ensure_path = path.join(name);
    if !ensure_path.exists() {
        create_dir_all(&ensure_path)?;
    }
    Ok(ensure_path)
}

fn traverse<T>(
    path: &PathBuf,
    f: impl for<'a> FnOnce(&PathBuf, fs::Metadata, &str) -> Result<Option<T>> + Copy,
) -> Result<Vec<T>> {
    let mut result = vec![];
    let paths = fs::read_dir(path)?;
    for path in paths {
        let path = path?.path();
        let name: &str = path
            .file_stem()
            .ok_or(Error::Custom("Could not find file name".to_string()))?
            .to_str()
            .unwrap();
        let md = fs::metadata(&path)?;
        if let Some(item) = f(&path, md, name)? {
            result.push(item);
        }
    }
    Ok(result)
}
